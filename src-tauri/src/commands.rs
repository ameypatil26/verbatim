use std::path::Path;

use base64::Engine;
use serde::Deserialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::dictation;
use crate::injection;
use crate::model_manager;
use crate::models::{
    AppSettings, CaptureResult, ChatMessage, ModelsReport, Note, NoteDraft, OcrResult,
    ServiceRuntimeStatus, SetupStatus, Transcription, UrlProgress,
};
use crate::screenshot;
use crate::settings as settings_store;
use crate::AppState;

#[tauri::command]
pub fn save_note(state: State<'_, AppState>, note: Note) -> Result<Note, String> {
    state.db.save_note(note)
}

#[tauri::command]
pub fn get_notes(state: State<'_, AppState>, query: Option<String>) -> Result<Vec<Note>, String> {
    state.db.get_notes(query.as_deref())
}

#[tauri::command]
pub fn list_notes(state: State<'_, AppState>, query: Option<String>) -> Result<Vec<Note>, String> {
    state.db.get_notes(query.as_deref())
}

#[tauri::command]
pub fn delete_note(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.db.delete_note(&id)
}

#[tauri::command]
pub fn list_messages(state: State<'_, AppState>, note_id: String) -> Result<Vec<ChatMessage>, String> {
    state.db.list_messages(&note_id)
}

#[tauri::command]
pub fn save_message(state: State<'_, AppState>, message: ChatMessage) -> Result<ChatMessage, String> {
    state.db.save_message(message)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppSettings {
    settings_store::load(&state.db)
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    if let Some(service) = state
        .hotkeys
        .lock()
        .map_err(|_| "hotkey lock poisoned".to_string())?
        .as_ref()
    {
        service.rebind(
            settings.hotkey_dictation.clone(),
            settings.hotkey_ocr.clone(),
        )?;
    }
    settings_store::save(&state.db, &settings)
}

#[tauri::command]
pub fn set_hotkeys(
    state: State<'_, AppState>,
    dictation: String,
    ocr: String,
) -> Result<AppSettings, String> {
    if let Some(service) = state
        .hotkeys
        .lock()
        .map_err(|_| "hotkey lock poisoned".to_string())?
        .as_ref()
    {
        service.rebind(dictation.clone(), ocr.clone())?;
    }
    let mut current = settings_store::load(&state.db);
    current.hotkey_dictation = dictation;
    current.hotkey_ocr = ocr;
    settings_store::save(&state.db, &current)
}

#[tauri::command]
pub fn dictate_start(app: AppHandle, state: State<'_, AppState>, source: Option<String>) -> Result<(), String> {
    let requested = source.unwrap_or_else(|| state.db.get_settings().audio_source);
    dictation::start(&app, &requested)
}

#[tauri::command]
pub async fn dictate_stop(app: AppHandle) -> Result<String, String> {
    dictation::stop_and_process(app).await
}

#[tauri::command]
pub async fn screenshot_ocr(app: AppHandle) -> Result<CaptureResult, String> {
    screenshot::capture_and_ocr(app).await
}

#[tauri::command]
pub async fn capture_region_ocr(app: AppHandle) -> Result<CaptureResult, String> {
    screenshot::capture_and_ocr(app).await
}

#[tauri::command]
pub fn undo_last_dictation() -> Result<(), String> {
    injection::undo_last_injection()
}

#[tauri::command]
pub fn check_models(app: AppHandle) -> ModelsReport {
    model_manager::check(&app)
}

#[tauri::command]
pub fn get_setup_status(app: AppHandle) -> SetupStatus {
    model_manager::setup_status(&app)
}

#[tauri::command]
pub async fn download_model(app: AppHandle, model: String) -> Result<(), String> {
    model_manager::download(app, &model).await
}

#[tauri::command]
pub fn delete_model(app: AppHandle, model: String) -> Result<(), String> {
    model_manager::delete(&app, &model)
}

#[tauri::command]
pub fn service_status(state: State<'_, AppState>) -> Vec<ServiceRuntimeStatus> {
    state.services.statuses()
}

#[tauri::command]
pub fn restart_service(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.services.restart(&name)
}

#[tauri::command]
pub async fn transcribe_path(
    state: State<'_, AppState>,
    path: String,
    language: Option<String>,
) -> Result<Transcription, String> {
    let settings = state.db.get_settings();
    let response = post_file(
        &state,
        &format!("{}/transcribe", settings.asr_url.trim_end_matches('/')),
        Path::new(&path),
        "file",
        Some(("language", language.unwrap_or(settings.language))),
        None,
    )
    .await?;
    parse_json(response, "ASR response").await
}

#[tauri::command]
pub async fn ocr_path(
    state: State<'_, AppState>,
    path: String,
    masks: Option<serde_json::Value>,
) -> Result<OcrResult, String> {
    let settings = state.db.get_settings();
    let response = post_file(
        &state,
        &format!("{}/ocr", settings.ocr_url.trim_end_matches('/')),
        Path::new(&path),
        "file",
        None,
        masks,
    )
    .await?;
    parse_json(response, "OCR response").await
}

#[tauri::command]
pub async fn ocr_pdf_path(
    state: State<'_, AppState>,
    path: String,
    masks: Option<serde_json::Value>,
) -> Result<OcrResult, String> {
    let settings = state.db.get_settings();
    let response = post_file(
        &state,
        &format!("{}/ocr-pdf", settings.ocr_url.trim_end_matches('/')),
        Path::new(&path),
        "file",
        None,
        masks,
    )
    .await?;
    parse_json(response, "PDF OCR response").await
}

#[tauri::command]
pub async fn searchable_pdf_path(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let settings = state.db.get_settings();
    let file = read_file_limited(Path::new(&path))?;
    let file_name = Path::new(&path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document.pdf")
        .to_string();
    let part = reqwest::multipart::Part::bytes(file)
        .file_name(file_name)
        .mime_str("application/pdf")
        .map_err(|error| format!("PDF part: {error}"))?;
    let form = reqwest::multipart::Form::new().part("file", part);
    let response = state
        .http
        .post(format!("{}/pdf-searchable", settings.ocr_url.trim_end_matches('/')))
        .multipart(form)
        .send()
        .await
        .map_err(|error| format!("OCR service unavailable: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Searchable PDF failed ({status}): {body}"));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Read searchable PDF: {error}"))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[tauri::command]
pub fn read_image_preview(path: String) -> Result<String, String> {
    let bytes = read_file_limited(Path::new(&path))?;
    if bytes.len() > 24 * 1024 * 1024 {
        return Err("Image is too large to preview (limit 24 MB).".into());
    }
    let extension = Path::new(&path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png")
        .to_ascii_lowercase();
    let mime = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        _ => "image/png",
    };
    Ok(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: Option<String>,
}

#[tauri::command]
pub async fn process_url(app: AppHandle, state: State<'_, AppState>, url: String) -> Result<NoteDraft, String> {
    let settings = state.db.get_settings();
    emit_url_progress(&app, "asr", 8, "Fetching media and transcribing locally");
    let response = state
        .http
        .post(format!("{}/transcribe", settings.asr_url.trim_end_matches('/')))
        .json(&json!({ "url": url, "language": settings.language }))
        .send()
        .await
        .map_err(|error| format!("ASR service unavailable: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Transcription failed ({status}): {body}"));
    }
    let transcription: Transcription = response
        .json()
        .await
        .map_err(|error| format!("ASR response: {error}"))?;
    emit_url_progress(&app, "ollama", 68, "Structuring notes with Ollama");
    let prompt = format!(
        "You are a note-taker. Convert this raw transcript into a clean Markdown document with: # Title, ## Key Points (bulleted), ## Detailed Notes (paragraphs), ## Action Items (if any). Preserve timestamps as [MM:SS] inline. Do not add information not in the transcript.\n\nTranscript:\n{}",
        transcription.text
    );
    let ollama_response = state
        .http
        .post(format!("{}/api/generate", settings.ollama_url.trim_end_matches('/')))
        .json(&json!({
            "model": settings.ollama_model,
            "prompt": prompt,
            "stream": false,
            "options": { "temperature": 0.2 }
        }))
        .send()
        .await;
    let markdown = match ollama_response {
        Ok(response) if response.status().is_success() => {
            response
                .json::<OllamaResponse>()
                .await
                .ok()
                .and_then(|value| value.response)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| format!("# Transcript\n\n{}", transcription.text))
        }
        _ => format!("# Transcript\n\n{}", transcription.text),
    };
    emit_url_progress(&app, "done", 100, "Note ready");
    let title = markdown
        .lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .map(str::trim)
        .unwrap_or("Imported media")
        .to_string();
    Ok(NoteDraft {
        title,
        transcript: transcription.text,
        markdown,
        duration: transcription.duration,
        source: url,
        source_type: "url".into(),
    })
}

fn emit_url_progress(app: &AppHandle, stage: &str, progress: u8, message: &str) {
    let _ = app.emit(
        "url:progress",
        UrlProgress {
            stage: stage.into(),
            progress,
            message: message.into(),
        },
    );
}

async fn post_file(
    state: &AppState,
    url: &str,
    path: &Path,
    field: &str,
    text_field: Option<(&str, String)>,
    masks: Option<serde_json::Value>,
) -> Result<reqwest::Response, String> {
    let bytes = read_file_limited(path)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("upload.bin")
        .to_string();
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("application/octet-stream")
        .map_err(|error| format!("Multipart part: {error}"))?;
    let mut form = reqwest::multipart::Form::new().part(field.to_string(), part);
    if let Some((name, value)) = text_field {
        form = form.text(name.to_string(), value);
    }
    if let Some(masks) = masks {
        form = form.text("masks", serde_json::to_string(&masks).unwrap_or_else(|_| "[]".into()));
    }
    let response = state
        .http
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|error| format!("Local service unavailable: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Local service error ({status}): {body}"));
    }
    Ok(response)
}

async fn parse_json<T: for<'de> Deserialize<'de>>(response: reqwest::Response, label: &str) -> Result<T, String> {
    response.json().await.map_err(|error| format!("{label}: {error}"))
}

fn read_file_limited(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = std::fs::metadata(path).map_err(|error| format!("Open {}: {error}", path.display()))?;
    if metadata.len() > 2 * 1024 * 1024 * 1024 {
        return Err("File exceeds the 2 GB limit.".into());
    }
    std::fs::read(path).map_err(|error| format!("Read {}: {error}", path.display()))
}

