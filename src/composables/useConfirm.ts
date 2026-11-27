// Sustituye a window.confirm (que cogni-vault usaba y no existe en el webview de
// Tauri) por un diálogo propio. App.vue pinta el diálogo una sola vez.
import { ref } from "vue";

export interface ConfirmRequest {
  title: string;
  message: string;
  confirmLabel?: string;
  destructive?: boolean;
  resolve: (ok: boolean) => void;
}

const pending = ref<ConfirmRequest | null>(null);

export function useConfirm() {
  function confirm(options: Omit<ConfirmRequest, "resolve">): Promise<boolean> {
    pending.value?.resolve(false);
    return new Promise((resolve) => {
      pending.value = { ...options, resolve };
    });
  }
  function answer(ok: boolean) {
    pending.value?.resolve(ok);
    pending.value = null;
  }
  return { pending, confirm, answer };
}
