import type { RequiredModelId } from './types';

export interface RequiredModel {
  id: RequiredModelId;
  name: string;
  short: string;
  role: string;
  size: string;
  source: string;
  accent: 'mint' | 'signal' | 'sand';
}

/**
 * The only model downloads Verbatim needs. Kept in sync with the Rust
 * `model_manager` constants; nothing here is fetched without explicit consent.
 */
export const REQUIRED_MODELS: RequiredModel[] = [
  {
    id: 'sensevoice',
    name: 'SenseVoice Small',
    short: 'SenseVoice',
    role: 'Speech-to-text · 5 languages',
    size: '~228 MB',
    source: 'Hugging Face',
    accent: 'mint'
  },
  {
    id: 'rapidocr',
    name: 'RapidOCR ONNX',
    short: 'RapidOCR',
    role: 'Text detection + recognition',
    size: '~15 MB',
    source: 'Bundled ONNX',
    accent: 'signal'
  },
  {
    id: 'qwen2.5-coder:7b',
    name: 'Qwen 2.5 Coder 7B',
    short: 'Qwen 7B',
    role: 'Structuring, proofreading, note chat',
    size: '~4.7 GB',
    source: 'Ollama',
    accent: 'sand'
  }
];

export function modelMeta(id: RequiredModelId): RequiredModel {
  return REQUIRED_MODELS.find((model) => model.id === id) ?? REQUIRED_MODELS[0];
}
