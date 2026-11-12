import type { ToastItem } from './types';

let nextId = 1;

export const toasts = $state<ToastItem[]>([]);

export function toast(message: string, kind: ToastItem['kind'] = 'info', ms = 3800): number {
  const id = nextId++;
  toasts.push({ id, message, kind });
  setTimeout(() => dismissToast(id), ms);
  return id;
}

export function dismissToast(id: number): void {
  const index = toasts.findIndex((item) => item.id === id);
  if (index >= 0) toasts.splice(index, 1);
}
