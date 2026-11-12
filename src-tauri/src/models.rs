use serde::{Deserialize, Serialize};

/// A saved Markdown note. Field names are camelCase on the Svelte side.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub title: String,
    pub source: String,
    pub source_type: String,
    pub markdown: String,
    pub transcript: String,
    pub duration: f64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Note {
    pub fn normalized(mut self) -> Self {
        if self.id.trim().is_empty() {
            self.id = uuid::Uuid::new_v4().to_string();
        }
        if self.title.trim().is_empty() {
            self.title = "Untitled note".to_string();
        }
        if self.created_at.trim().is_empty() {
            self.created_at = now_iso();
        }
        self.updated_at = now_iso();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub note_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    pub text: String,
    pub duration: f64,
    pub language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub confidence: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrCode {
    #[serde(rename = "type")]
    pub kind: String,
    pub data: String,
    pub rect: OcrRect,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult {
    pub text: String,
    #[serde(default)]
    pub boxes: Vec<OcrBox>,
    #[serde(default)]
    pub codes: Vec<OcrCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    #[serde(flatten)]
    pub ocr: OcrResult,
    pub image_data_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mask {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDraft {
    pub title: String,
    pub transcript: String,
    pub markdown: String,
    pub duration: f64,
    pub source: String,
    pub source_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlProgress {
    pub stage: String,
    pub progress: u8,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictationEvent {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRuntimeStatus {
    pub name: String,
    pub state: String,
    pub pid: Option<u32>,
    pub retries: u8,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub hotkey_dictation: String,
    pub hotkey_ocr: String,
    pub dictation_mode: String,
    pub audio_source: String,
    pub proofread: bool,
    pub language: String,
    pub default_format: String,
    pub ollama_url: String,
    pub ollama_model: String,
    pub asr_url: String,
    pub ocr_url: String,
    pub mcp_url: String,
    pub setup_complete: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey_dictation: "Ctrl+Shift+D".into(),
            hotkey_ocr: "Ctrl+Shift+S".into(),
            dictation_mode: "tap".into(),
            audio_source: "microphone".into(),
            proofread: true,
            language: "auto".into(),
            default_format: "md".into(),
            ollama_url: "http://127.0.0.1:11434".into(),
            ollama_model: "qwen2.5-coder:7b".into(),
            asr_url: "http://127.0.0.1:8001".into(),
            ocr_url: "http://127.0.0.1:8002".into(),
            mcp_url: "http://127.0.0.1:3900".into(),
            setup_complete: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatus {
    /// Stable key used by download_model: sensevoice | rapidocr | qwen2.5-coder:7b
    pub model: String,
    pub name: String,
    pub ready: bool,
    pub size_bytes: u64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsReport {
    pub models: Vec<ModelStatus>,
    pub ready: bool,
    pub ollama_installed: bool,
    pub data_dir: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub ready: bool,
    pub data_dir: String,
    pub models: Vec<SetupModel>,
    pub ollama_reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupModel {
    pub key: String,
    pub label: String,
    pub detail: String,
    pub installed: bool,
    pub loading: bool,
    pub size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    pub model: String,
    pub downloaded_mb: f64,
    pub total_mb: f64,
    pub status: String,
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
