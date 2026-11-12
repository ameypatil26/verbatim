use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::injection;
use crate::models::{DictationEvent, Transcription};
use crate::AppState;

pub fn emit_state(app: &AppHandle, state: &str, text: Option<String>, error: Option<String>, duration: Option<f64>, source: Option<String>) {
    let payload = DictationEvent {
        state: state.to_string(),
        text,
        error,
        duration,
        source,
    };
    let _ = app.emit("dictation:state", payload);
}

pub fn start(app: &AppHandle, source: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    let settings = state.db.get_settings();
    let requested = if source == "microphone" || source == "system" {
        source.to_string()
    } else {
        settings.audio_source.clone()
    };
    state.audio.start(&requested, app.clone())?;
    emit_state(app, "recording", None, None, None, Some(requested));
    Ok(())
}

pub async fn stop_and_process(app: AppHandle) -> Result<String, String> {
    let state = app.state::<AppState>();
    if !state.audio.is_recording() {
        return Err("No dictation recording is in progress.".into());
    }
    let audio = Arc::clone(&state.audio);
    let stopped = tokio::task::spawn_blocking(move || audio.stop())
        .await
        .map_err(|error| format!("audio join task: {error}"))??;
    let (path, source) = stopped;
    emit_state(&app, "processing", None, None, None, Some(source.clone()));

    let result = process_recording(&app, &path).await;
    let _ = std::fs::remove_file(&path);
    match result {
        Ok((text, duration)) => {
            let state = app.state::<AppState>();
            state.audio.set_last_text(&text);
            let _ = app.emit(
                "dictation:result",
                serde_json::json!({ "text": text, "duration": duration, "source": source }),
            );
            emit_state(&app, "idle", Some(text.clone()), None, Some(duration), Some(source));
            Ok(text)
        }
        Err(error) => {
            tracing::error!(error = %error, "dictation failed");
            emit_state(&app, "error", None, Some(error.clone()), None, Some(source));
            Err(error)
        }
    }
}

async fn process_recording(app: &AppHandle, path: &Path) -> Result<(String, f64), String> {
    let state = app.state::<AppState>();
    let settings = state.db.get_settings();
    let bytes = std::fs::read(path).map_err(|error| format!("read recording: {error}"))?;
    if bytes.is_empty() {
        return Err("The recording was empty. Check your microphone and try again.".into());
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("dictation.wav")
        .to_string();
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name)
        .mime_str("audio/wav")
        .map_err(|error| format!("audio part: {error}"))?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("language", settings.language.clone());
    let url = format!("{}/transcribe", settings.asr_url.trim_end_matches('/'));
    let response = state
        .http
        .post(url)
        .multipart(form)
        .send()
        .await
        .map_err(|error| format!("ASR service unavailable: {error}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("ASR failed ({status}): {body}"));
    }
    let transcription: Transcription = response
        .json()
        .await
        .map_err(|error| format!("ASR response: {error}"))?;
    let mut text = transcription.text.trim().to_string();
    if text.is_empty() {
        return Err("No speech was detected in the recording.".into());
    }

    if settings.proofread {
        if let Ok(proofread) = proofread_text(app, &text).await {
            if !proofread.trim().is_empty() {
                text = proofread.trim().to_string();
            }
        }
    }

    if let Err(error) = injection::type_text(&text) {
        tracing::error!(error = %error, "text was transcribed but could not be typed");
        emit_state(
            app,
            "error",
            Some(text.clone()),
            Some(format!("Transcribed, but could not type into the focused app: {error}")),
            Some(transcription.duration),
            None,
        );
    }
    Ok((text, transcription.duration))
}

async fn proofread_text(app: &AppHandle, transcript: &str) -> Result<String, String> {
    #[derive(Deserialize)]
    struct OllamaResponse {
        response: Option<String>,
    }
    let state = app.state::<AppState>();
    let settings = state.db.get_settings();
    let prompt = format!(
        "Fix punctuation, capitalization, and obvious speech-recognition slips in the text below. Preserve the wording and language. Return only the corrected text, with no commentary.\n\n{transcript}"
    );
    let response = state
        .http
        .post(format!("{}/api/generate", settings.ollama_url.trim_end_matches('/')))
        .timeout(Duration::from_secs(120))
        .json(&serde_json::json!({
            "model": settings.ollama_model,
            "prompt": prompt,
            "system": "You are a precise local proofreader. Never add facts that are not present.",
            "stream": false,
            "options": { "temperature": 0.2 }
        }))
        .send()
        .await
        .map_err(|error| format!("Ollama unavailable: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Ollama returned {}", response.status()));
    }
    let data: OllamaResponse = response.json().await.map_err(|error| format!("Ollama response: {error}"))?;
    Ok(data.response.unwrap_or_default())
}
