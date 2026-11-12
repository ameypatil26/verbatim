import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { mergeSettings, DEFAULT_SETTINGS } from './settings';
import type {
  AppSettings,
  CaptureResult,
  ChatMessage,
  DictationEvent,
  HealthSnapshot,
  Mask,
  ModelDownloadProgress,
  ModelsReport,
  RequiredModelId,
  Note,
  NoteDraft,
  OcrResult,
  ServiceHealth,
  ServiceRuntimeStatus,
  SetupStatus,
  Transcription,
  UrlProgress
} from './types';

export const isTauri =
  typeof window !== 'undefined' && Object.prototype.hasOwnProperty.call(window, '__TAURI_INTERNALS__');

export const SUMMARIZE_PROMPT =
  'You are a note-taker. Convert this raw transcript into a clean Markdown document with: # Title, ## Key Points (bulleted), ## Detailed Notes (paragraphs), ## Action Items (if any). Preserve timestamps as [MM:SS] inline. Do not add information not in the transcript.';

export const CHAT_SYSTEM_PROMPT =
  'Answer only from the provided transcript. If the answer is not present, say so.';

const LS_SETTINGS = 'verbatim.settings.v1';
const LS_NOTES = 'verbatim.notes.v1';
const LS_MESSAGES = 'verbatim.messages.v1';
const LS_WELCOME = 'verbatim.welcome.v1';

function nowIso(): string {
  return new Date().toISOString();
}

export function uid(): string {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) return crypto.randomUUID();
  return `id-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  if (error && typeof error === 'object' && 'message' in error) return String((error as { message: unknown }).message);
  try {
    return JSON.stringify(error);
  } catch {
    return 'Unknown error';
  }
}

async function fetchWithTimeout(url: string, init: RequestInit = {}, ms = 2500): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), ms);
  try {
    return await fetch(url, { ...init, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

async function parseError(response: Response): Promise<string> {
  try {
    const data = await response.json();
    if (data && typeof data.detail === 'string') return data.detail;
    return JSON.stringify(data);
  } catch {
    return `${response.status} ${response.statusText}`;
  }
}

// ---------------------------------------------------------------------------
// Settings (SQLite through Tauri, localStorage in the browser preview)
// ---------------------------------------------------------------------------

export async function loadSettings(): Promise<AppSettings> {
  if (isTauri) {
    try {
      const stored = await invoke<Partial<AppSettings> | null>('get_settings');
      return mergeSettings(stored);
    } catch {
      // A first run may not have a settings row yet.
      return { ...DEFAULT_SETTINGS };
    }
  }
  try {
    const raw = localStorage.getItem(LS_SETTINGS);
    return mergeSettings(raw ? JSON.parse(raw) : null);
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

export async function saveSettings(settings: AppSettings): Promise<AppSettings> {
  const merged = mergeSettings(settings);
  if (isTauri) {
    await invoke('save_settings', { settings: merged });
  } else {
    localStorage.setItem(LS_SETTINGS, JSON.stringify(merged));
  }
  return merged;
}

export async function saveSettingsPatch(patch: Partial<AppSettings>): Promise<AppSettings> {
  const current = await loadSettings();
  return saveSettings({ ...current, ...patch });
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

async function healthOne(
  base: string,
  kind: 'asr' | 'ocr' | 'ollama'
): Promise<ServiceHealth> {
  const started = performance.now();
  try {
    const url = kind === 'ollama' ? `${base}/api/tags` : `${base}/health`;
    const response = await fetchWithTimeout(url, { method: 'GET' }, 1800);
    const latencyMs = Math.round(performance.now() - started);
    if (!response.ok) {
      return { status: 'down', error: `${response.status} ${response.statusText}`, latencyMs };
    }
    const data = await response.json();
    if (kind === 'ollama') {
      return { status: 'ok', model: `${Array.isArray(data.models) ? data.models.length : 0} models`, latencyMs };
    }
    return {
      status: data.status === 'ok' ? 'ok' : 'degraded',
      model: data.model,
      loaded: Boolean(data.loaded),
      latencyMs
    };
  } catch (error) {
    return {
      status: 'down',
      error: errorMessage(error),
      latencyMs: Math.round(performance.now() - started)
    };
  }
}

export async function checkHealth(settings: AppSettings): Promise<HealthSnapshot> {
  const [asr, ocr, ollama] = await Promise.all([
    healthOne(settings.asrUrl, 'asr'),
    healthOne(settings.ocrUrl, 'ocr'),
    healthOne(settings.ollamaUrl, 'ollama')
  ]);
  return { asr, ocr, ollama, checkedAt: Date.now() };
}

export function onServiceStatus(callback: (status: ServiceRuntimeStatus[]) => void): () => void {
  if (!isTauri) return () => {};
  let disposed = false;
  let unlisten: UnlistenFn | undefined;
  listen<ServiceRuntimeStatus[]>('service:status', (event) => callback(event.payload)).then((fn) => {
    if (disposed) fn();
    else unlisten = fn;
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

export async function serviceRuntimeStatus(): Promise<ServiceRuntimeStatus[]> {
  if (!isTauri) return [];
  return invoke<ServiceRuntimeStatus[]>('service_status');
}

export async function restartService(name: 'asr' | 'ocr' | 'mcp'): Promise<void> {
  if (!isTauri) throw new Error('Service restart is available in the desktop app.');
  await invoke('restart_service', { name });
}

// ---------------------------------------------------------------------------
// Notes
// ---------------------------------------------------------------------------

function localNotes(): Note[] {
  try {
    const notes = JSON.parse(localStorage.getItem(LS_NOTES) || '[]') as Note[];
    return Array.isArray(notes) ? notes : [];
  } catch {
    return [];
  }
}

function writeLocalNotes(notes: Note[]): void {
  localStorage.setItem(LS_NOTES, JSON.stringify(notes));
}

function welcomeNote(): Note {
  const createdAt = nowIso();
  return {
    id: uid(),
    title: 'Welcome to Verbatim',
    source: 'Verbatim',
    sourceType: 'manual',
    duration: 0,
    createdAt,
    updatedAt: createdAt,
    tags: ['welcome'],
    transcript: '',
    markdown: `# Welcome to Verbatim

**Every word, on your machine.**

Verbatim turns speech, media, images, and PDFs into clean Markdown without sending a byte to a cloud service.

## Key Points

- **Dictation** types into any focused app with \`Ctrl+Shift+D\`.
- **Notes** converts audio, video, YouTube, and TikTok links into structured Markdown.
- **Vision** runs OCR, QR/barcode detection, batch exports, and searchable PDFs.
- **Everything local** lives in SQLite under \`~/.verbatim/\`.

## Detailed Notes

Start a dictation from the Dictation tab. Drop a file in Notes, or paste a URL. Drop images or PDFs in Vision and select the text in the result panel.

The three localhost services are independent: ASR on port 8001, OCR on 8002, and Ollama on 11434. The desktop shell starts ASR and OCR for you, then unloads their models when idle.

## Action Items

- [ ] Run the first-launch setup to install local models
- [ ] Try \`Ctrl+Shift+D\` in another app
- [ ] Make a searchable PDF from a scan
`
  };
}

export async function listNotes(query = ''): Promise<Note[]> {
  if (isTauri) {
    return invoke<Note[]>('get_notes', { query: query || null });
  }
  if (!localStorage.getItem(LS_WELCOME)) {
    const seeded = localNotes();
    if (!seeded.length) writeLocalNotes([welcomeNote()]);
    localStorage.setItem(LS_WELCOME, '1');
  }
  const notes = localNotes().sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
  const needle = query.trim().toLowerCase();
  if (!needle) return notes;
  return notes.filter((note) =>
    `${note.title}\n${note.markdown}\n${note.transcript}`.toLowerCase().includes(needle)
  );
}

export async function saveNote(note: Note): Promise<Note> {
  const normalized = { ...note, updatedAt: nowIso() };
  if (isTauri) {
    return invoke<Note>('save_note', { note: normalized });
  }
  const notes = localNotes();
  const index = notes.findIndex((item) => item.id === normalized.id);
  const stored = { ...normalized, updatedAt: nowIso() };
  if (index >= 0) notes[index] = stored;
  else notes.push(stored);
  writeLocalNotes(notes);
  return stored;
}

export async function deleteNote(id: string): Promise<void> {
  if (isTauri) {
    await invoke('delete_note', { id });
    return;
  }
  writeLocalNotes(localNotes().filter((note) => note.id !== id));
  const messages = readLocalMessages();
  delete messages[id];
  localStorage.setItem(LS_MESSAGES, JSON.stringify(messages));
}

function readLocalMessages(): Record<string, ChatMessage[]> {
  try {
    const parsed = JSON.parse(localStorage.getItem(LS_MESSAGES) || '{}');
    return parsed && typeof parsed === 'object' ? parsed : {};
  } catch {
    return {};
  }
}

export async function listMessages(noteId: string): Promise<ChatMessage[]> {
  if (isTauri) return invoke<ChatMessage[]>('list_messages', { noteId });
  return readLocalMessages()[noteId] ?? [];
}

export async function saveMessage(message: ChatMessage): Promise<ChatMessage> {
  if (isTauri) return invoke<ChatMessage>('save_message', { message });
  const messages = readLocalMessages();
  const list = messages[message.noteId] ?? [];
  list.push({ ...message, id: Date.now() });
  messages[message.noteId] = list;
  localStorage.setItem(LS_MESSAGES, JSON.stringify(messages));
  return { ...message, id: Date.now() };
}

// ---------------------------------------------------------------------------
// ASR / OCR direct service calls
// ---------------------------------------------------------------------------

export async function transcribeFile(
  file: Blob,
  settings: AppSettings,
  language = settings.language,
  fileName = 'audio.wav'
): Promise<Transcription> {
  const form = new FormData();
  form.append('file', file, fileName);
  form.append('language', language || 'auto');
  const response = await fetchWithTimeout(
    `${settings.asrUrl}/transcribe`,
    { method: 'POST', body: form },
    20 * 60 * 1000
  );
  if (!response.ok) throw new Error(await parseError(response));
  return (await response.json()) as Transcription;
}

export async function transcribePath(
  path: string,
  settings: AppSettings,
  language = settings.language
): Promise<Transcription> {
  if (!isTauri) throw new Error('File paths require the desktop app.');
  return invoke<Transcription>('transcribe_path', { path, language });
}

export async function transcribeUrl(
  url: string,
  settings: AppSettings,
  language = settings.language
): Promise<Transcription> {
  const response = await fetchWithTimeout(
    `${settings.asrUrl}/transcribe`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url, language })
    },
    20 * 60 * 1000
  );
  if (!response.ok) throw new Error(await parseError(response));
  return (await response.json()) as Transcription;
}

export async function ocrFile(file: Blob, settings: AppSettings, masks: Mask[] = []): Promise<OcrResult> {
  const form = new FormData();
  form.append('file', file, (file as File).name || 'image.png');
  form.append('masks', JSON.stringify(masks));
  const response = await fetchWithTimeout(
    `${settings.ocrUrl}/ocr`,
    { method: 'POST', body: form },
    10 * 60 * 1000
  );
  if (!response.ok) throw new Error(await parseError(response));
  return normalizeOcr(await response.json());
}

export async function ocrPdf(file: Blob, settings: AppSettings, masks: Mask[] = []): Promise<OcrResult> {
  const form = new FormData();
  form.append('file', file, (file as File).name || 'document.pdf');
  form.append('masks', JSON.stringify(masks));
  const response = await fetchWithTimeout(
    `${settings.ocrUrl}/ocr-pdf`,
    { method: 'POST', body: form },
    30 * 60 * 1000
  );
  if (!response.ok) throw new Error(await parseError(response));
  return normalizeOcr(await response.json());
}

export async function ocrPath(path: string, settings: AppSettings, masks: Mask[] = []): Promise<OcrResult> {
  if (!isTauri) throw new Error('File paths require the desktop app.');
  return normalizeOcr(await invoke('ocr_path', { path, masks }));
}

export async function ocrPdfPath(path: string, settings: AppSettings, masks: Mask[] = []): Promise<OcrResult> {
  if (!isTauri) throw new Error('File paths require the desktop app.');
  return normalizeOcr(await invoke('ocr_pdf_path', { path, masks }));
}

function normalizeOcr(value: Partial<OcrResult>): OcrResult {
  return {
    text: value.text ?? '',
    boxes: value.boxes ?? [],
    codes: value.codes ?? [],
    pageCount: value.pageCount
  };
}

export async function searchablePdf(file: Blob, settings: AppSettings): Promise<Blob> {
  const form = new FormData();
  form.append('file', file, (file as File).name || 'document.pdf');
  const response = await fetchWithTimeout(
    `${settings.ocrUrl}/pdf-searchable`,
    { method: 'POST', body: form },
    30 * 60 * 1000
  );
  if (!response.ok) throw new Error(await parseError(response));
  return response.blob();
}

export async function searchablePdfPath(path: string): Promise<string> {
  if (!isTauri) throw new Error('File paths require the desktop app.');
  return invoke<string>('searchable_pdf_path', { path });
}

export async function captureRegionOcr(settings: AppSettings): Promise<CaptureResult> {
  if (isTauri) return invoke<CaptureResult>('screenshot_ocr');
  const result = await ocrFile(await captureBrowserFrame(), settings);
  return result;
}

async function captureBrowserFrame(): Promise<File> {
  if (!navigator.mediaDevices?.getDisplayMedia) {
    throw new Error('Screen capture is not available in this browser. Use the desktop app.');
  }
  const stream = await navigator.mediaDevices.getDisplayMedia({ video: true });
  try {
    const video = document.createElement('video');
    video.srcObject = stream;
    await video.play();
    await new Promise((resolve) => setTimeout(resolve, 250));
    const canvas = document.createElement('canvas');
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;
    canvas.getContext('2d')?.drawImage(video, 0, 0);
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
    if (!blob) throw new Error('Could not capture a frame.');
    return new File([blob], 'capture.png', { type: 'image/png' });
  } finally {
    stream.getTracks().forEach((track) => track.stop());
  }
}

// ---------------------------------------------------------------------------
// Ollama
// ---------------------------------------------------------------------------

async function ollamaGenerate(prompt: string, settings: AppSettings, system?: string): Promise<string> {
  const response = await fetchWithTimeout(
    `${settings.ollamaUrl}/api/generate`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: settings.ollamaModel,
        prompt,
        system,
        stream: false,
        options: { temperature: 0.2 }
      })
    },
    15 * 60 * 1000
  );
  if (!response.ok) throw new Error(await parseError(response));
  const data = await response.json();
  return String(data.response || '').trim();
}

export async function summarizeText(transcript: string, settings: AppSettings): Promise<string> {
  return ollamaGenerate(`${SUMMARIZE_PROMPT}\n\nTranscript:\n${transcript}`, settings);
}

export async function proofreadText(text: string, settings: AppSettings): Promise<string> {
  return ollamaGenerate(
    `Fix punctuation, capitalization, and obvious speech-recognition slips in the text below. Preserve the wording and language. Return only the corrected text, with no commentary.\n\n${text}`,
    settings,
    'You are a precise local proofreader. Never add facts that are not present.'
  );
}

export async function chatWithNote(
  question: string,
  note: Note,
  history: ChatMessage[],
  settings: AppSettings
): Promise<string> {
  const messages = [
    { role: 'system', content: `${CHAT_SYSTEM_PROMPT}\n\nTranscript:\n${note.transcript || note.markdown}` },
    ...history
      .filter((message) => message.role === 'user' || message.role === 'assistant')
      .slice(-8)
      .map((message) => ({ role: message.role, content: message.content })),
    { role: 'user', content: question }
  ];
  const response = await fetchWithTimeout(
    `${settings.ollamaUrl}/api/chat`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: settings.ollamaModel,
        messages,
        stream: false,
        options: { temperature: 0.2 }
      })
    },
    15 * 60 * 1000
  );
  if (!response.ok) throw new Error(await parseError(response));
  const data = await response.json();
  return String(data.message?.content || '').trim();
}

export async function testOllama(settings: AppSettings): Promise<{ ok: boolean; models: string[]; error?: string }> {
  try {
    const response = await fetchWithTimeout(`${settings.ollamaUrl}/api/tags`, {}, 5000);
    if (!response.ok) return { ok: false, models: [], error: await parseError(response) };
    const data = await response.json();
    return {
      ok: true,
      models: (data.models || []).map((model: { name?: string }) => model.name).filter(Boolean)
    };
  } catch (error) {
    return { ok: false, models: [], error: errorMessage(error) };
  }
}

// ---------------------------------------------------------------------------
// URL pipeline
// ---------------------------------------------------------------------------

export function onUrlProgress(callback: (progress: UrlProgress) => void): () => void {
  if (!isTauri) return () => {};
  let disposed = false;
  let unlisten: UnlistenFn | undefined;
  listen<UrlProgress>('url:progress', (event) => callback(event.payload)).then((fn) => {
    if (disposed) fn();
    else unlisten = fn;
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

export async function processUrl(
  url: string,
  settings: AppSettings,
  onProgress?: (progress: UrlProgress) => void
): Promise<NoteDraft> {
  if (isTauri) {
    const unlisten = onUrlProgress((progress) => onProgress?.(progress));
    try {
      return await invoke<NoteDraft>('process_url', { url });
    } finally {
      unlisten();
    }
  }
  onProgress?.({ stage: 'asr', progress: 20, message: 'Downloading audio with yt-dlp fallback' });
  const transcription = await transcribeUrl(url, settings);
  onProgress?.({ stage: 'ollama', progress: 70, message: 'Structuring notes locally' });
  let markdown = '';
  try {
    markdown = await summarizeText(transcription.text, settings);
  } catch {
    markdown = `# Transcript\n\n${transcription.text}`;
  }
  const firstLine = markdown.match(/^#\s+(.+)$/m)?.[1] || 'Imported media';
  return {
    title: firstLine,
    transcript: transcription.text,
    markdown,
    duration: transcription.duration,
    source: url,
    sourceType: 'url'
  };
}

// ---------------------------------------------------------------------------
// Dictation events / commands
// ---------------------------------------------------------------------------

export function onDictation(callback: (event: DictationEvent) => void): () => void {
  if (!isTauri) return () => {};
  let disposed = false;
  let unlisten: UnlistenFn | undefined;
  listen<DictationEvent>('dictation:state', (event) => callback(event.payload)).then((fn) => {
    if (disposed) fn();
    else unlisten = fn;
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

export function onDictationLevel(callback: (level: number) => void): () => void {
  if (!isTauri) return () => {};
  let disposed = false;
  let unlisten: UnlistenFn | undefined;
  listen<number>('dictation:level', (event) => callback(event.payload)).then((fn) => {
    if (disposed) fn();
    else unlisten = fn;
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

export async function startDictation(source: 'microphone' | 'system'): Promise<void> {
  if (!isTauri) throw new Error('Dictation into other apps is available in the desktop app.');
  await invoke('dictate_start', { source });
}

export async function stopDictation(): Promise<string> {
  if (!isTauri) throw new Error('Dictation into other apps is available in the desktop app.');
  return invoke<string>('dictate_stop');
}

export async function undoLastDictation(): Promise<void> {
  if (!isTauri) throw new Error('Undo is available in the desktop app.');
  await invoke('undo_last_dictation');
}

// ---------------------------------------------------------------------------
// Setup and models
// ---------------------------------------------------------------------------

export async function getSetupStatus(): Promise<SetupStatus> {
  if (isTauri) return invoke<SetupStatus>('get_setup_status');
  return {
    ready: false,
    dataDir: '~/.verbatim',
    ollamaReachable: false,
    models: [
      { key: 'asr', label: 'SenseVoice ASR', detail: 'Local speech model', installed: false, loading: false, sizeBytes: 0 },
      { key: 'ocr', label: 'RapidOCR', detail: 'Detection + recognition', installed: false, loading: false, sizeBytes: 0 },
      { key: 'llm', label: 'qwen2.5-coder:7b', detail: 'Local structuring + chat', installed: false, loading: false, sizeBytes: 0 }
    ],
    message: 'Browser preview: launch the desktop shell to install and supervise local models.'
  };
}

export async function checkModels(): Promise<ModelsReport> {
  if (!isTauri) {
    return {
      models: [
        { model: 'sensevoice', name: 'SenseVoice ASR', ready: false, sizeBytes: 0, detail: 'Speech recognition · ~228 MB' },
        { model: 'rapidocr', name: 'RapidOCR ONNX', ready: false, sizeBytes: 0, detail: 'OCR models · ~15 MB' },
        { model: 'qwen2.5-coder:7b', name: 'Qwen 2.5 Coder 7B', ready: false, sizeBytes: 0, detail: 'Not pulled yet · ~4.7 GB' }
      ],
      ready: false,
      ollamaInstalled: false,
      dataDir: '~/.verbatim',
      message: 'Browser preview: launch the desktop shell to install and supervise local models.'
    };
  }
  return invoke<ModelsReport>('check_models');
}

export function onModelProgress(callback: (event: ModelDownloadProgress) => void): () => void {
  if (!isTauri) return () => {};
  let disposed = false;
  let unlisten: UnlistenFn | undefined;
  listen<ModelDownloadProgress>('model-download-progress', (event) => callback(event.payload)).then((fn) => {
    if (disposed) fn();
    else unlisten = fn;
  });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

export async function downloadModel(model: RequiredModelId): Promise<void> {
  if (!isTauri) throw new Error('Model downloads are available in the desktop app.');
  await invoke('download_model', { model });
}

export async function deleteModel(model: RequiredModelId): Promise<void> {
  if (!isTauri) throw new Error('Model management is available in the desktop app.');
  await invoke('delete_model', { model });
}

export async function setHotkeys(dictation: string, ocr: string): Promise<AppSettings> {
  if (!isTauri) return loadSettings();
  return invoke<AppSettings>('set_hotkeys', { dictation, ocr });
}

export async function readImagePreview(path: string): Promise<string> {
  if (!isTauri) throw new Error('File paths require the desktop app.');
  return invoke<string>('read_image_preview', { path });
}

// ---------------------------------------------------------------------------
// Download helpers
// ---------------------------------------------------------------------------

export function downloadBlob(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export function downloadText(filename: string, text: string, mime = 'text/plain;charset=utf-8'): void {
  downloadBlob(new Blob([text], { type: mime }), filename);
}

export async function copyText(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}

export function formatBytes(bytes: number): string {
  if (!bytes) return '—';
  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(unit > 1 ? 1 : 0)} ${units[unit]}`;
}

export function formatDuration(seconds: number): string {
  if (!seconds || !Number.isFinite(seconds)) return '—';
  const total = Math.round(seconds);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return h > 0 ? `${h}h ${m}m` : m > 0 ? `${m}m ${s}s` : `${s}s`;
}

export function formatDate(value: string): string {
  try {
    const date = new Date(value);
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(date);
  } catch {
    return value;
  }
}

export function noteFromDraft(draft: NoteDraft): Note {
  const now = nowIso();
  return {
    id: uid(),
    title: draft.title || 'Untitled note',
    source: draft.source,
    sourceType: draft.sourceType,
    markdown: draft.markdown,
    transcript: draft.transcript,
    duration: draft.duration || 0,
    createdAt: now,
    updatedAt: now,
    tags: []
  };
}

export function noteFromTranscript(
  transcription: Transcription,
  source: string,
  markdown: string,
  sourceType: Note['sourceType'] = 'file'
): Note {
  const title = markdown.match(/^#\s+(.+)$/m)?.[1] || source.split(/[\\/]/).pop() || 'Untitled note';
  return {
    id: uid(),
    title,
    source,
    sourceType,
    markdown,
    transcript: transcription.text,
    duration: transcription.duration || 0,
    createdAt: nowIso(),
    updatedAt: nowIso(),
    tags: []
  };
}

export function csvEscape(value: string): string {
  return `"${(value || '').replace(/"/g, '""')}"`;
}
