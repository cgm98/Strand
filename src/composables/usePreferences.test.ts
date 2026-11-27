import { describe, expect, test } from "bun:test";
import { AUTOSAVE_DELAYS, usePreferences } from "./usePreferences";

// `usePreferences()` es un singleton de módulo (como useAppearance/useToast):
// todas las llamadas devuelven el mismo estado, así que estos tests se fijan
// en que los cambios se reflejen y persistan, no en el valor inicial (que
// depende de qué había en localStorage *antes* de que el módulo se cargara).
describe("usePreferences", () => {
  test("setAutosaveDelay cambia el estado y lo persiste en localStorage", () => {
    const preferences = usePreferences();
    preferences.setAutosaveDelay(1500);
    expect(preferences.state.autosaveDelayMs).toBe(1500);
    expect(JSON.parse(localStorage.getItem("notas.preferences")!)).toEqual({ autosaveDelayMs: 1500 });
  });

  test("todas las opciones declaradas son válidas para setAutosaveDelay", () => {
    const preferences = usePreferences();
    for (const { ms } of AUTOSAVE_DELAYS) {
      preferences.setAutosaveDelay(ms);
      expect(preferences.state.autosaveDelayMs).toBe(ms);
    }
  });

  test("dos llamadas a usePreferences() comparten el mismo estado (singleton)", () => {
    const a = usePreferences();
    const b = usePreferences();
    a.setAutosaveDelay(300);
    expect(b.state.autosaveDelayMs).toBe(300);
  });
});
