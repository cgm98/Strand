// Preferencias generales de la app (a diferencia de Apariencia, que vive en
// <datos>/appearance.json): solo comodidad de la interfaz, no hace falta que
// sobreviva una reinstalación, así que con localStorage alcanza.
import { reactive, readonly } from "vue";

const KEY = "notas.preferences";

export const AUTOSAVE_DELAYS = [
  { ms: 300, label: "Rápido (0,3 s)" },
  { ms: 650, label: "Normal (0,65 s)" },
  { ms: 1500, label: "Lento (1,5 s)" },
  { ms: 3000, label: "Muy lento (3 s)" },
] as const;
const DEFAULT_AUTOSAVE_DELAY_MS = 650;

interface Preferences {
  autosaveDelayMs: number;
}

function read(): Preferences {
  try {
    const raw = JSON.parse(localStorage.getItem(KEY) || "null") as Partial<Preferences> | null;
    const ms = raw?.autosaveDelayMs;
    return { autosaveDelayMs: AUTOSAVE_DELAYS.some((d) => d.ms === ms) ? ms! : DEFAULT_AUTOSAVE_DELAY_MS };
  } catch {
    return { autosaveDelayMs: DEFAULT_AUTOSAVE_DELAY_MS };
  }
}

const state = reactive(read());

function write() {
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
  } catch {
    // Preferencia de comodidad: si no se puede guardar, se queda solo en esta sesión.
  }
}

export function usePreferences() {
  return {
    state: readonly(state),
    setAutosaveDelay(ms: number) {
      state.autosaveDelayMs = ms;
      write();
    },
  };
}
