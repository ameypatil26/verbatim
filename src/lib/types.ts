export type TabId = 'dictation' | 'notes' | 'vision' | 'settings';

export type HealthState = 'ok' | 'down' | 'checking';

export interface ServiceHealth {
  status: 'ok' | 'degraded' | 'down' | 'checking';
  model?: string;
  loaded?: boolean;
  latencyMs?: number;
  error?: string;
}

export interface HealthSnapshot {
  asr: ServiceHealth;
  ocr: ServiceHealth;
  ollama: ServiceHealth;
  checkedAt: number;
}

export interface Note {
  id: string;
  title: string;
  source: string;
  sourceType: 'file' | 'url' | 'dictation' | 'manual';
  markdown: string;
  transcript: string;
  duration: number;
  createdAt: string;
  updatedAt: string;
  tags: string[];
}

export interface ChatMessage {
  id?: number;
  noteId: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  createdAt: string;
}

export interface Transcription {
  text: string;
  duration: number;
  language: string;
}

export interface OcrBox {
  x: number;
  y: number;
  w: number;
  h: number;
  confidence: number;
  page?: number;
}

export interface OcrCode {
  type: string;
  data: string;
  rect: { x: number; y: number; w: number; h: number };
  page?: number;
}

export interface OcrResult {
  text: string;
  boxes: OcrBox[];
  codes: OcrCode[];
  pageCount?: number;
}

export interface Mask {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface AppSettings {
  hotkeyDictation: string;
  hotkeyOcr: string;
  dictationMode: 'tap' | 'push';
  audioSource: 'microphone' | 'system';
  proofread: boolean;
  language: string;
  defaultFormat: 'md' | 'txt' | 'csv';
  ollamaUrl: string;
  ollamaModel: string;
  asrUrl: string;
  ocrUrl: string;
  mcpUrl: string;
  setupComplete: boolean;
}

export interface ModelState {
  key: 'asr' | 'ocr' | 'llm';
  label: string;
  detail: string;
  installed: boolean;
  loading: boolean;
  sizeBytes: number;
  version?: string;
  error?: string;
}

export type RequiredModelId = 'sensevoice' | 'rapidocr' | 'qwen2.5-coder:7b';

export interface ModelCheck {
  model: RequiredModelId;
  name: string;
  ready: boolean;
  sizeBytes: number;
  detail: string;
}

export interface ModelsReport {
  models: ModelCheck[];
  ready: boolean;
  ollamaInstalled: boolean;
  dataDir: string;
  message?: string;
}

export interface ModelDownloadProgress {
  model: RequiredModelId;
  downloadedMb: number;
  totalMb: number;
  status: 'downloading' | 'done' | 'error';
}

export interface SetupStatus {
  ready: boolean;
  dataDir: string;
  models: ModelState[];
  ollamaReachable: boolean;
  message?: string;
}

export interface HotkeysError {
  message: string;
}

export interface ServiceRuntimeStatus {
  name: string;
  state: 'running' | 'starting' | 'failed' | 'stopped' | 'external';
  pid?: number;
  retries: number;
  message?: string;
}

export interface NoteDraft {
  title: string;
  transcript: string;
  markdown: string;
  duration: number;
  source: string;
  sourceType: 'file' | 'url' | 'manual';
}

export interface DictationEvent {
  state: 'idle' | 'recording' | 'processing' | 'error';
  text?: string;
  error?: string;
  duration?: number;
  source?: string;
}

export interface UrlProgress {
  stage: string;
  progress: number;
  message: string;
}

export interface ToastItem {
  id: number;
  message: string;
  kind: 'info' | 'success' | 'error';
}

export interface CaptureResult extends OcrResult {
  imageDataUrl?: string;
}
