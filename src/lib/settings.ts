import type { AppSettings } from './types';

export const DEFAULT_SETTINGS: AppSettings = {
  hotkeyDictation: 'Ctrl+Shift+D',
  hotkeyOcr: 'Ctrl+Shift+S',
  dictationMode: 'tap',
  audioSource: 'microphone',
  proofread: true,
  language: 'auto',
  defaultFormat: 'md',
  ollamaUrl: 'http://127.0.0.1:11434',
  ollamaModel: 'qwen2.5-coder:7b',
  asrUrl: 'http://127.0.0.1:8001',
  ocrUrl: 'http://127.0.0.1:8002',
  mcpUrl: 'http://127.0.0.1:3900',
  setupComplete: false
};

export function mergeSettings(value: Partial<AppSettings> | null | undefined): AppSettings {
  const merged = { ...DEFAULT_SETTINGS, ...(value ?? {}) };
  merged.asrUrl = normalizeLocalUrl(merged.asrUrl, 8001);
  merged.ocrUrl = normalizeLocalUrl(merged.ocrUrl, 8002);
  merged.ollamaUrl = normalizeLocalUrl(merged.ollamaUrl, 11434);
  return merged;
}

export function normalizeLocalUrl(url: string, fallbackPort: number): string {
  const trimmed = (url || '').trim().replace(/\/+$/, '');
  if (!trimmed) return `http://127.0.0.1:${fallbackPort}`;
  if (!/^https?:\/\//i.test(trimmed)) return `http://${trimmed}`;
  return trimmed;
}

export function hotkeyIsValid(value: string): boolean {
  return /^(Ctrl|Alt|Shift|Meta)(\+(Ctrl|Alt|Shift|Meta))*\+[A-Z0-9]$/i.test(value.trim());
}
