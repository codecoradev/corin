/// Toast notification store — simple reactive toast manager.

export type ToastType = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  type: ToastType;
  message: string;
}

let toasts = $state<Toast[]>([]);
let nextId = 0;

/** Show a toast. Auto-dismisses after duration (ms). */
function push(type: ToastType, message: string, duration = 3000) {
  const id = ++nextId;
  toasts = [...toasts, { id, type, message }];
  if (duration > 0) {
    setTimeout(() => dismiss(id), duration);
  }
}

/** Remove a toast by id. */
function dismiss(id: number) {
  toasts = toasts.filter((t) => t.id !== id);
}

export const toastStore = {
  get list() { return toasts; },
  dismiss,
  success: (msg: string, dur?: number) => push('success', msg, dur),
  error: (msg: string, dur?: number) => push('error', msg, dur ?? 6000),
  info: (msg: string, dur?: number) => push('info', msg, dur),
  clear: () => { toasts = []; },
};
