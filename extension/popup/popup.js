/* global chrome */
'use strict';

const elements = {
  detect: document.getElementById('detect'),
  detectDot: document.getElementById('detect-dot'),
  detectTitle: document.getElementById('detect-title'),
  detectDetail: document.getElementById('detect-detail'),
  retry: document.getElementById('retry'),
  enabled: document.getElementById('enabled'),
  floating: document.getElementById('floating'),
  asrPort: document.getElementById('asr-port'),
  ocrPort: document.getElementById('ocr-port'),
  ollamaPort: document.getElementById('ollama-port'),
  asrDot: document.getElementById('asr-dot'),
  ocrDot: document.getElementById('ocr-dot'),
  ollamaDot: document.getElementById('ollama-dot'),
  asrState: document.getElementById('asr-state'),
  ocrState: document.getElementById('ocr-state'),
  ollamaState: document.getElementById('ollama-state'),
  asrPortLabel: document.getElementById('asr-port-label'),
  ocrPortLabel: document.getElementById('ocr-port-label'),
  ollamaPortLabel: document.getElementById('ollama-port-label'),
  capture: document.getElementById('capture'),
  save: document.getElementById('save'),
  saved: document.getElementById('saved')
};

const DEFAULT_SETTINGS = {
  enabled: true,
  asrPort: 8001,
  ocrPort: 8002,
  ollamaPort: 11434,
  showFloating: true
};

let settings = { ...DEFAULT_SETTINGS };
let detected = false;

function setDot(element, state) {
  element.className = `dot ${state}`;
}

function applySettings(next) {
  settings = { ...settings, ...next };
  elements.enabled.checked = settings.enabled !== false;
  elements.floating.checked = settings.showFloating !== false;
  elements.asrPort.value = settings.asrPort;
  elements.ocrPort.value = settings.ocrPort;
  elements.ollamaPort.value = settings.ollamaPort;
  elements.asrPortLabel.textContent = `:${settings.asrPort}`;
  elements.ocrPortLabel.textContent = `:${settings.ocrPort}`;
  elements.ollamaPortLabel.textContent = `:${settings.ollamaPort}`;
}

function renderStatus(payload) {
  if (!payload || !payload.ok) {
    setDetectDown(payload && payload.error ? payload.error : 'Verbatim not detected.');
    return;
  }
  applySettings(payload.settings || {});
  const health = payload.health || {};
  setDot(elements.asrDot, health.asr ? 'ok' : 'down');
  setDot(elements.ocrDot, health.ocr ? 'ok' : 'down');
  setDot(elements.ollamaDot, health.ollama ? 'ok' : 'down');
  elements.asrState.textContent = health.asr ? 'ready' : 'offline';
  elements.ocrState.textContent = health.ocr ? 'ready' : 'offline';
  elements.ollamaState.textContent = health.ollama ? 'ready' : 'offline';
  detected = Boolean(health.asr && health.ocr);
  elements.capture.disabled = !detected || !settings.enabled;
  if (detected) {
    elements.detect.classList.remove('down');
    setDot(elements.detectDot, 'ok');
    elements.detectTitle.textContent = 'Verbatim detected';
    elements.detectDetail.textContent = 'Local services are running on this machine';
  } else {
    setDetectDown('Verbatim not detected. Start the desktop app and try again.');
  }
  if (settings.enabled === false) {
    elements.detectDetail.textContent = 'Extension is turned off';
  }
}

function setDetectDown(message) {
  detected = false;
  elements.detect.classList.add('down');
  setDot(elements.detectDot, 'down');
  elements.detectTitle.textContent = 'Verbatim not detected';
  elements.detectDetail.textContent = message;
  elements.capture.disabled = true;
  setDot(elements.asrDot, 'down');
  setDot(elements.ocrDot, 'down');
  setDot(elements.ollamaDot, 'down');
  elements.asrState.textContent = 'offline';
  elements.ocrState.textContent = 'offline';
  elements.ollamaState.textContent = 'offline';
}

async function refresh() {
  setDot(elements.detectDot, 'checking');
  elements.detectTitle.textContent = 'Checking Verbatim…';
  elements.detectDetail.textContent = 'Looking for localhost services';
  try {
    const response = await chrome.runtime.sendMessage({ type: 'get-status' });
    renderStatus(response);
  } catch (error) {
    setDetectDown(String(error));
  }
}

async function saveSettings(patch, showSaved = true) {
  try {
    const response = await chrome.runtime.sendMessage({
      type: 'save-settings',
      settings: { ...settings, ...patch }
    });
    if (response && response.ok) {
      applySettings(response.settings);
      elements.capture.disabled = !detected || !settings.enabled;
      if (showSaved) {
        elements.saved.textContent = 'Saved';
        setTimeout(() => (elements.saved.textContent = ''), 1600);
      }
      return true;
    }
  } catch (error) {
    elements.saved.textContent = 'Save failed';
    setTimeout(() => (elements.saved.textContent = ''), 2200);
  }
  return false;
}

elements.enabled.addEventListener('change', () => {
  void saveSettings({ enabled: elements.enabled.checked });
});
elements.floating.addEventListener('change', () => {
  void saveSettings({ showFloating: elements.floating.checked });
});
elements.retry.addEventListener('click', () => void refresh());
elements.save.addEventListener('click', async () => {
  const asrPort = Number(elements.asrPort.value);
  const ocrPort = Number(elements.ocrPort.value);
  const ollamaPort = Number(elements.ollamaPort.value);
  if (![asrPort, ocrPort, ollamaPort].every((port) => Number.isInteger(port) && port > 0 && port < 65536)) {
    elements.saved.textContent = 'Invalid port';
    return;
  }
  const ok = await saveSettings({ asrPort, ocrPort, ollamaPort });
  if (ok) await refresh();
});
elements.capture.addEventListener('click', async () => {
  if (!detected) return;
  elements.capture.textContent = 'Capturing…';
  try {
    await chrome.runtime.sendMessage({ type: 'capture-viewport' });
  } finally {
    window.close();
  }
});

void refresh();
setInterval(() => {
  if (!document.hidden) void refresh();
}, 15000);
