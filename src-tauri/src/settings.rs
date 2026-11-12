use crate::db::Database;
use crate::models::AppSettings;

pub fn load(db: &Database) -> AppSettings {
    db.get_settings()
}

pub fn save(db: &Database, settings: &AppSettings) -> Result<AppSettings, String> {
    let mut normalized = settings.clone();
    normalized.hotkey_dictation = normalized.hotkey_dictation.trim().to_string();
    normalized.hotkey_ocr = normalized.hotkey_ocr.trim().to_string();
    if normalized.hotkey_dictation.is_empty() {
        normalized.hotkey_dictation = "Ctrl+Shift+D".into();
    }
    if normalized.hotkey_ocr.is_empty() {
        normalized.hotkey_ocr = "Ctrl+Shift+S".into();
    }
    normalized.asr_url = normalize_url(&normalized.asr_url, 8001);
    normalized.ocr_url = normalize_url(&normalized.ocr_url, 8002);
    normalized.ollama_url = normalize_url(&normalized.ollama_url, 11434);
    if normalized.ollama_model.trim().is_empty() {
        normalized.ollama_model = "qwen2.5-coder:7b".into();
    }
    normalized.dictation_mode = if normalized.dictation_mode == "push" {
        "push".into()
    } else {
        "tap".into()
    };
    if normalized.audio_source != "system" {
        normalized.audio_source = "microphone".into();
    }
    db.save_settings(&normalized)?;
    Ok(normalized)
}

fn normalize_url(value: &str, fallback_port: u16) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return format!("http://127.0.0.1:{fallback_port}");
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}
