/**
 * Verbatim MV3 service worker.
 *
 * All processing requests go to the user's localhost microservices. The only
 * outbound traffic from this extension is to 127.0.0.1/localhost.
 */

interface Settings {
  enabled: boolean;
  asrPort: number;
  ocrPort: number;
  ollamaPort: number;
  showFloating: boolean;
}

interface Health {
  asr: boolean;
  ocr: boolean;
  ollama: boolean;
}

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

interface OcrResponse {
  text: string;
  boxes: Array<{ x: number; y: number; w: number; h: number; confidence: number }>;
  codes?: Array<{ type: string; data: string }>;
}

const DEFAULTS: Settings = {
  enabled: true,
  asrPort: 8001,
  ocrPort: 8002,
  ollamaPort: 11434,
  showFloating: true
};

const CONTEXT_OCR = 'verbatim-ocr';
const CONTEXT_CAPTURE = 'verbatim-capture';
const CONTEXT_TRANSCRIBE = 'verbatim-transcribe';

const MENU_ITEMS: chrome.contextMenus.CreateProperties[] = [
  {
    id: CONTEXT_OCR,
    title: 'Verbatim OCR',
    contexts: ['image', 'page', 'selection']
  },
  {
    id: CONTEXT_CAPTURE,
    title: 'Verbatim Capture',
    contexts: ['selection', 'page']
  },
  {
    id: CONTEXT_TRANSCRIBE,
    title: 'Verbatim Transcribe',
    contexts: ['page', 'video', 'audio']
  }
];

chrome.runtime.onInstalled.addListener(() => {
  void setupMenus();
  void updateBadge();
});

chrome.runtime.onStartup.addListener(() => {
  void setupMenus();
  void updateBadge();
});

chrome.contextMenus.onClicked.addListener((info, tab) => {
  void handleContextMenu(info, tab);
});

chrome.runtime.onMessage.addListener((message: unknown, sender, sendResponse) => {
  void handleMessage(message, sender)
    .then((value) => sendResponse(value))
    .catch((error) => sendResponse({ ok: false, error: friendlyError(error) }));
  return true;
});

chrome.storage.onChanged.addListener((changes, area) => {
  if (area === 'sync' && Object.keys(changes).length) {
    void updateBadge();
  }
});

async function setupMenus(): Promise<void> {
  await new Promise<void>((resolve) => chrome.contextMenus.removeAll(() => resolve()));
  for (const item of MENU_ITEMS) {
    chrome.contextMenus.create(item);
  }
}

async function getSettings(): Promise<Settings> {
  const stored = await chrome.storage.sync.get(DEFAULTS);
  return {
    enabled: stored.enabled !== false,
    asrPort: Number(stored.asrPort) || DEFAULTS.asrPort,
    ocrPort: Number(stored.ocrPort) || DEFAULTS.ocrPort,
    ollamaPort: Number(stored.ollamaPort) || DEFAULTS.ollamaPort,
    showFloating: stored.showFloating !== false
  };
}

async function probe(url: string, parseJson = false): Promise<boolean> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 1800);
  try {
    const response = await fetch(url, { signal: controller.signal, cache: 'no-store' });
    if (!response.ok) return false;
    if (parseJson) await response.json();
    return true;
  } catch {
    return false;
  } finally {
    clearTimeout(timer);
  }
}

async function checkHealth(settings: Settings): Promise<Health> {
  const [asr, ocr, ollama] = await Promise.all([
    probe(`http://127.0.0.1:${settings.asrPort}/health`),
    probe(`http://127.0.0.1:${settings.ocrPort}/health`),
    probe(`http://127.0.0.1:${settings.ollamaPort}/api/tags`, true)
  ]);
  return { asr, ocr, ollama };
}

async function updateBadge(): Promise<void> {
  const settings = await getSettings();
  if (!settings.enabled) {
    await chrome.action.setBadgeText({ text: 'OFF' });
    await chrome.action.setBadgeBackgroundColor({ color: '#555d6b' });
    return;
  }
  const health = await checkHealth(settings);
  const ready = health.asr && health.ocr;
  await chrome.action.setBadgeText({ text: ready ? '' : '!' });
  await chrome.action.setBadgeBackgroundColor({ color: ready ? '#4ecca3' : '#e94560' });
  await chrome.action.setTitle({
    title: ready ? 'Verbatim — local services ready' : 'Verbatim not detected — start the desktop app'
  });
}

async function handleContextMenu(info: chrome.contextMenus.OnClickData, tab?: chrome.tabs.Tab): Promise<void> {
  if (!tab?.id) return;
  const settings = await getSettings();
  if (!settings.enabled) {
    await showPanel(tab.id, 'Verbatim is off', 'Turn the extension back on from the Verbatim toolbar popup.', 'info');
    return;
  }
  if (info.menuItemId === CONTEXT_OCR) {
    if (info.mediaType === 'image' && info.srcUrl) {
      await sendToTab(tab.id, { type: 'verbatim:context', action: 'ocr-image', srcUrl: info.srcUrl });
    } else {
      await sendToTab(tab.id, { type: 'verbatim:context', action: 'select-region' });
    }
  } else if (info.menuItemId === CONTEXT_CAPTURE) {
    await sendToTab(tab.id, { type: 'verbatim:context', action: 'capture-text' });
  } else if (info.menuItemId === CONTEXT_TRANSCRIBE) {
    await sendToTab(tab.id, { type: 'verbatim:context', action: 'transcribe' });
  }
}

async function handleMessage(message: any, sender: chrome.runtime.MessageSender): Promise<unknown> {
  const settings = await getSettings();
  const tabId = sender.tab?.id;

  switch (message?.type) {
    case 'get-status': {
      const health = await checkHealth(settings);
      await updateBadge();
      return {
        ok: true,
        settings,
        health,
        detected: health.asr && health.ocr
      };
    }
    case 'save-settings': {
      const next: Settings = {
        enabled: message.settings?.enabled !== false,
        asrPort: Number(message.settings?.asrPort) || DEFAULTS.asrPort,
        ocrPort: Number(message.settings?.ocrPort) || DEFAULTS.ocrPort,
        ollamaPort: Number(message.settings?.ollamaPort) || DEFAULTS.ollamaPort,
        showFloating: message.settings?.showFloating !== false
      };
      await chrome.storage.sync.set(next);
      await updateBadge();
      return { ok: true, settings: next };
    }
    case 'capture-viewport': {
      const targetTab = tabId ? { id: tabId } : (await chrome.tabs.query({ active: true, currentWindow: true }))[0];
      if (!targetTab?.id) throw new Error('No active tab.');
      await handleCaptureViewport(targetTab.id, settings);
      return { ok: true };
    }
    case 'ocr-region': {
      if (!tabId) throw new Error('No source tab.');
      return handleOcrRegion(tabId, message.rect, Number(message.dpr) || 1, settings);
    }
    case 'ocr-image': {
      if (!tabId) throw new Error('No source tab.');
      return handleOcrRegion(tabId, message.rect, Number(message.dpr) || 1, settings);
    }
    case 'capture-text': {
      if (!tabId) throw new Error('No source tab.');
      return handleCaptureText(tabId, String(message.text || ''), settings);
    }
    case 'transcribe-media': {
      if (!tabId) throw new Error('No source tab.');
      return handleTranscribe(tabId, String(message.mediaUrl || ''), settings);
    }
    case 'copy': {
      if (!tabId) return { ok: true };
      await sendToTab(tabId, { type: 'verbatim:copy', text: String(message.text || '') });
      return { ok: true };
    }
    default:
      return { ok: false, error: 'Unknown message.' };
  }
}

async function handleOcrRegion(tabId: number, rect: Rect | undefined, dpr: number, settings: Settings): Promise<unknown> {
  await showPanel(tabId, 'Verbatim OCR', 'Reading pixels locally…', 'loading');
  try {
    const health = await checkHealth(settings);
    if (!health.ocr) throw new Error('Verbatim not detected. Start the desktop app, then try again.');
    const blob = await captureVisibleTab(tabId, rect, dpr);
    const result = await postOcr(blob, settings);
    const text = result.text || 'No text was detected in that region.';
    await sendToTab(tabId, { type: 'verbatim:copy', text: result.text });
    await showPanel(tabId, 'Verbatim OCR', text, 'result');
    return { ok: true, result };
  } catch (error) {
    await showPanel(tabId, 'Verbatim OCR', friendlyError(error), 'error');
    return { ok: false, error: friendlyError(error) };
  }
}

async function handleCaptureViewport(tabId: number, settings: Settings): Promise<void> {
  await showPanel(tabId, 'Verbatim OCR', 'Capturing the visible tab locally…', 'loading');
  try {
    const health = await checkHealth(settings);
    if (!health.ocr) throw new Error('Verbatim not detected. Start the desktop app, then try again.');
    const blob = await captureVisibleTab(tabId);
    const result = await postOcr(blob, settings);
    await sendToTab(tabId, { type: 'verbatim:copy', text: result.text });
    await showPanel(tabId, 'Verbatim OCR · viewport', result.text || 'No text detected.', 'result');
  } catch (error) {
    await showPanel(tabId, 'Verbatim OCR', friendlyError(error), 'error');
  }
}

async function handleCaptureText(tabId: number, text: string, settings: Settings): Promise<unknown> {
  await showPanel(tabId, 'Verbatim Capture', 'Summarizing the visible page locally…', 'loading');
  try {
    if (!text.trim()) throw new Error('No visible text was found on this page.');
    const health = await checkHealth(settings);
    if (!health.ollama) throw new Error('Verbatim not detected. Start the desktop app and Ollama, then try again.');
    const response = await fetch(`http://127.0.0.1:${settings.ollamaPort}/api/generate`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        model: 'qwen2.5-coder:7b',
        stream: false,
        options: { temperature: 0.2 },
        prompt:
          'Summarize the visible page text below as compact Markdown with a short title and bullet points. Do not invent information.\n\n' +
          text.slice(0, 18000)
      })
    });
    if (!response.ok) throw new Error(`Ollama returned ${response.status}. Is qwen2.5-coder:7b pulled?`);
    const data = (await response.json()) as { response?: string };
    const summary = String(data.response || '').trim();
    if (!summary) throw new Error('The local model returned an empty summary.');
    await showPanel(tabId, 'Verbatim Capture · summary', summary, 'result');
    return { ok: true, summary };
  } catch (error) {
    await showPanel(tabId, 'Verbatim Capture', friendlyError(error), 'error');
    return { ok: false, error: friendlyError(error) };
  }
}

async function handleTranscribe(tabId: number, mediaUrl: string, settings: Settings): Promise<unknown> {
  await showPanel(tabId, 'Verbatim Transcribe', 'Downloading and transcribing locally…', 'loading');
  try {
    const health = await checkHealth(settings);
    if (!health.asr) throw new Error('Verbatim not detected. Start the desktop app, then try again.');
    if (!mediaUrl) throw new Error('No media player or supported page URL was detected.');
    const response = await fetch(`http://127.0.0.1:${settings.asrPort}/transcribe`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url: mediaUrl, language: 'auto' })
    });
    if (!response.ok) {
      const detail = await response.text();
      throw new Error(`ASR returned ${response.status}: ${detail.slice(0, 300)}`);
    }
    const data = (await response.json()) as { text?: string; duration?: number };
    const text = String(data.text || '').trim();
    if (!text) throw new Error('No speech was detected.');
    await sendToTab(tabId, { type: 'verbatim:copy', text });
    await showPanel(tabId, 'Verbatim Transcribe', text, 'result');
    return { ok: true, text, duration: data.duration };
  } catch (error) {
    await showPanel(tabId, 'Verbatim Transcribe', friendlyError(error), 'error');
    return { ok: false, error: friendlyError(error) };
  }
}

async function captureVisibleTab(tabId: number, rect?: Rect, dpr = 1): Promise<Blob> {
  const tab = await chrome.tabs.get(tabId);
  const dataUrl = await chrome.tabs.captureVisibleTab(tab.windowId, { format: 'png' });
  const original = await (await fetch(dataUrl)).blob();
  if (!rect || rect.w <= 1 || rect.h <= 1) return original;

  const bitmap = await createImageBitmap(original);
  try {
    const canvas = new OffscreenCanvas(
      Math.max(1, Math.round(rect.w * dpr)),
      Math.max(1, Math.round(rect.h * dpr))
    );
    const context = canvas.getContext('2d');
    if (!context) throw new Error('Could not create an image canvas.');
    context.drawImage(
      bitmap,
      Math.round(rect.x * dpr),
      Math.round(rect.y * dpr),
      Math.round(rect.w * dpr),
      Math.round(rect.h * dpr),
      0,
      0,
      Math.max(1, Math.round(rect.w * dpr)),
      Math.max(1, Math.round(rect.h * dpr))
    );
    return await canvas.convertToBlob({ type: 'image/png' });
  } finally {
    bitmap.close();
  }
}

async function postOcr(blob: Blob, settings: Settings): Promise<OcrResponse> {
  const form = new FormData();
  form.append('file', blob, 'extension-capture.png');
  form.append('masks', '[]');
  const response = await fetch(`http://127.0.0.1:${settings.ocrPort}/ocr`, {
    method: 'POST',
    body: form
  });
  if (!response.ok) {
    const detail = await response.text();
    throw new Error(`OCR returned ${response.status}: ${detail.slice(0, 300)}`);
  }
  return (await response.json()) as OcrResponse;
}

async function showPanel(tabId: number, title: string, text: string, tone: 'loading' | 'result' | 'error' | 'info'): Promise<void> {
  await sendToTab(tabId, { type: 'verbatim:show-panel', title, text, tone });
}

async function sendToTab(tabId: number, payload: unknown): Promise<void> {
  try {
    await chrome.tabs.sendMessage(tabId, payload);
  } catch {
    // Restricted pages (chrome://, PDF viewer) cannot host content scripts.
  }
}

function friendlyError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes('Failed to fetch') || message.includes('NetworkError')) {
    return 'Verbatim not detected. Start the desktop app, then try again.';
  }
  return message;
}
