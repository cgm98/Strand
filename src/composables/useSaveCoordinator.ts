// Portado de cogni-vault. Coordina el autoguardado por nota:
// - agrupa ediciones seguidas en un solo guardado (debounce);
// - nunca lanza dos guardados a la vez de la misma nota;
// - si llega una edición durante el guardado, vuelve a guardar antes de dar el OK;
// - reintenta una vez los fallos transitorios y nunca falla en silencio.
// Cambio respecto al original: `isRetryable` evita reintentar errores que no se
// arreglan solos (CONFLICT, VALIDATION) y se guarda el error para la interfaz.
import { computed, getCurrentScope, onScopeDispose, ref } from "vue";
import type { SaveStatus } from "../types";

interface SaveOptions {
  exists: (id: string) => boolean;
  save: (id: string) => Promise<unknown>;
  /** Función (no un número fijo) para que un cambio de preferencia aplique sin reiniciar. */
  delay?: () => number;
  isRetryable?: (error: unknown) => boolean;
  onSaveSuccess?: (id: string) => void;
  onSaveError?: (id: string, error: unknown) => void;
}

export function useSaveCoordinator(options: SaveOptions) {
  const saveStates = ref<Record<string, SaveStatus>>({});
  const saveErrors = ref<Record<string, unknown>>({});
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const revisions = new Map<string, number>();
  const flights = new Map<string, Promise<boolean>>();
  const hasUnsavedChanges = computed(() => Object.values(saveStates.value).some((s) => s !== "saved"));

  function cancelScheduledSave(id: string) {
    clearTimeout(timers.get(id));
    timers.delete(id);
  }

  function markDirty(id: string) {
    revisions.set(id, (revisions.get(id) || 0) + 1);
    saveStates.value[id] = "dirty";
  }

  function scheduleSave(id: string) {
    markDirty(id);
    cancelScheduledSave(id);
    timers.set(
      id,
      setTimeout(() => void persist(id), options.delay?.() ?? 650),
    );
  }

  async function persist(id: string, settings: { notify?: boolean } = {}): Promise<boolean> {
    cancelScheduledSave(id);
    const pending = flights.get(id);
    if (pending) return pending;
    if (!options.exists(id) || saveStates.value[id] === undefined || saveStates.value[id] === "saved") return true;

    const operation = (async () => {
      while (options.exists(id)) {
        const revision = revisions.get(id) || 0;
        saveStates.value[id] = "saving";
        let failure: unknown = null;
        for (let attempt = 0; attempt < 2; attempt++) {
          try {
            await options.save(id);
            failure = null;
            break;
          } catch (error) {
            failure = error;
            if (options.isRetryable && !options.isRetryable(error)) break;
          }
        }
        if (failure) {
          saveStates.value[id] = "error";
          saveErrors.value[id] = failure;
          options.onSaveError?.(id, failure);
          return false;
        }
        // Llegó otra edición mientras se guardaba: guardarla antes de dar el OK.
        if (revision !== (revisions.get(id) || 0)) {
          cancelScheduledSave(id);
          continue;
        }
        saveStates.value[id] = "saved";
        delete saveErrors.value[id];
        if (settings.notify) options.onSaveSuccess?.(id);
        return true;
      }
      return false;
    })();
    flights.set(id, operation);
    try {
      return await operation;
    } finally {
      flights.delete(id);
    }
  }

  /** Guarda ya una nota aunque no tenga cambios pendientes (Ctrl+S). */
  async function saveNow(id: string) {
    markDirty(id);
    return persist(id, { notify: true });
  }

  async function flushUnsaved(): Promise<boolean> {
    const ids = Object.keys(saveStates.value).filter((id) => saveStates.value[id] !== "saved");
    const results = await Promise.all(ids.map((id) => persist(id)));
    return results.every(Boolean) && !hasUnsavedChanges.value;
  }

  function clearSaveTracking(id?: string) {
    for (const key of id ? [id] : Object.keys(saveStates.value)) {
      cancelScheduledSave(key);
      revisions.delete(key);
      delete saveStates.value[key];
      delete saveErrors.value[key];
    }
  }

  if (getCurrentScope())
    onScopeDispose(() => {
      for (const timer of timers.values()) clearTimeout(timer);
      timers.clear();
    });

  return {
    saveStates,
    saveErrors,
    hasUnsavedChanges,
    markDirty,
    scheduleSave,
    persist,
    saveNow,
    flushUnsaved,
    clearSaveTracking,
  };
}
