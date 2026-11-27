import { ref } from "vue";

export type ToastType = "success" | "error" | "info" | "warning";
export interface ToastMessage {
  id: number;
  type: ToastType;
  text: string;
}

// Estado global: todos los componentes comparten la misma pila de avisos.
const toasts = ref<ToastMessage[]>([]);
let seq = 0;

function dismiss(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id);
}

function show(text: string, type: ToastType = "info") {
  const id = ++seq;
  toasts.value.push({ id, type, text });
  // Los errores se quedan más tiempo: hay que poder leerlos.
  setTimeout(() => dismiss(id), type === "error" ? 8000 : 3500);
}

export function useToast() {
  return {
    toasts,
    show,
    dismiss,
    success: (msg: string) => show(msg, "success"),
    error: (msg: string) => show(msg, "error"),
    info: (msg: string) => show(msg, "info"),
    warning: (msg: string) => show(msg, "warning"),
  };
}
