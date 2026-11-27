// Apariencia: modo claro/oscuro, tema (incluido o del usuario) y fragmentos CSS.
// Ver src-tauri/src/appearance.rs y src/themes/.
//
// - Modo: <html data-color-scheme="light|dark">. «Sistema» se resuelve aquí y se
//   sigue con matchMedia, así el CSS (y los temas) solo miran el atributo.
// - Tema y fragmentos: <style> al final de <body>, después de todo el CSS de la
//   app (también del que Vite añade en desarrollo). Orden: tema y luego fragmentos.
// - Caché en localStorage: el script de index.html la aplica antes del primer
//   pintado, para que no se vea un destello del tema predeterminado al arrancar.
// - Al volver a la ventana se releen los archivos: editar el CSS en otro
//   programa se ve sin reiniciar.
import { reactive, readonly } from "vue";
import { api, ApiError, type AppearanceSettings, type ColorScheme, type CssKind } from "../lib/api";
import { builtinThemes } from "../themes";

/** Misma clave que lee el script de index.html. */
const CACHE_KEY = "notas.appearance";

const state = reactive({
  themes: [] as string[],
  snippets: [] as string[],
  settings: { theme: null, colorScheme: "system", snippets: [] } as AppearanceSettings,
  /** Archivos que no se pudieron aplicar, con el motivo. Clave: id del tema o `snippet:<nombre>`. */
  errors: {} as Record<string, string>,
  loaded: false,
  /** Esquema efectivo («sistema» ya resuelto), para las muestras de color. */
  scheme: "light" as "light" | "dark",
});

/** `builtin:x` / `user:x`; un nombre sin prefijo (versión anterior) es del usuario. */
export function parseThemeId(id: string): { source: "builtin" | "user"; name: string } {
  if (id.startsWith("builtin:")) return { source: "builtin", name: id.slice(8) };
  return { source: "user", name: id.startsWith("user:") ? id.slice(5) : id };
}

// ---------- Modo claro / oscuro ----------
const systemDark = window.matchMedia("(prefers-color-scheme: dark)");
const resolveScheme = (mode: ColorScheme) => (mode === "system" ? (systemDark.matches ? "dark" : "light") : mode);
function applyScheme() {
  state.scheme = resolveScheme(state.settings.colorScheme);
  document.documentElement.dataset.colorScheme = state.scheme;
}
systemDark.addEventListener("change", applyScheme);

// ---------- Hojas de estilo ----------
const host = () => {
  let el = document.getElementById("user-appearance");
  if (!el) {
    el = document.createElement("div");
    el.id = "user-appearance";
    el.hidden = true;
    document.body.append(el);
  }
  return el;
};

async function themeCss(id: string): Promise<string | null> {
  const { source, name } = parseThemeId(id);
  if (source === "builtin") return builtinThemes.find((t) => t.id === name)?.css ?? null;
  return state.themes.includes(name) ? api.readAppearanceCss("theme", name) : null;
}

function writeCache(sheets: Array<{ id: string; css: string }>) {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ colorScheme: state.settings.colorScheme, sheets }));
  } catch {
    /* Sin caché solo se pierde el arranque sin destello. */
  }
}

let applying = 0;
async function apply() {
  applyScheme();
  const request = ++applying;
  const errors: Record<string, string> = {};
  const fail = (id: string, e: unknown) => (errors[id] = e instanceof ApiError ? e.message : String(e));
  const wanted: Array<{ id: string; load: () => Promise<string | null> }> = [];
  const theme = state.settings.theme;
  if (theme) wanted.push({ id: theme, load: () => themeCss(theme) });
  for (const name of state.settings.snippets)
    if (state.snippets.includes(name)) wanted.push({ id: `snippet:${name}`, load: () => api.readAppearanceCss("snippet", name) });

  const sheets = (
    await Promise.all(
      wanted.map(async ({ id, load }) => {
        try {
          const css = await load();
          return css === null ? null : { id, css };
        } catch (e) {
          fail(id, e);
          return null;
        }
      }),
    )
  ).filter((s) => s !== null);
  if (request !== applying) return; // llegó otra petición más nueva mientras se leía
  state.errors = errors;
  writeCache(sheets);
  // Sin cambios (lo normal al volver a la ventana): no tocar el DOM.
  const current = Array.from(host().children) as HTMLStyleElement[];
  if (current.length === sheets.length && current.every((el, i) => el.dataset.appearance === sheets[i].id && el.textContent === sheets[i].css)) return;
  host().replaceChildren(
    ...sheets.map(({ id, css }) => {
      const style = document.createElement("style");
      style.dataset.appearance = id;
      style.textContent = css;
      return style;
    }),
  );
}

async function reload() {
  const result = await api.getAppearance();
  state.themes = result.themes;
  state.snippets = result.snippets;
  state.settings = result.settings;
  state.loaded = true;
  // Un tema incluido que ya no existe (se quitaron Sepia, Nord y Solarizado) se
  // cambia por el predeterminado y se guarda, para que el diálogo no quede sin
  // ninguna opción marcada.
  const theme = result.settings.theme;
  if (theme) {
    const { source, name } = parseThemeId(theme);
    if (source === "builtin" && !builtinThemes.some((t) => t.id === name)) {
      await update({ theme: null }).catch(() => undefined);
      return;
    }
  }
  await apply();
}

async function update(patch: Partial<AppearanceSettings>) {
  const previous = state.settings;
  const next = { ...previous, ...patch };
  state.settings = next;
  await apply();
  try {
    await api.setAppearance(next);
  } catch (e) {
    // Si no se pudo guardar, volver a lo que había para no mentir sobre el estado.
    state.settings = previous;
    await apply();
    throw e;
  }
}

/** Copia un tema incluido a la carpeta del usuario y lo activa. Devuelve el nombre del archivo. */
/** Guarda un archivo de tema nuevo en la carpeta del usuario y lo activa. */
async function saveUserTheme(displayName: string, css: string) {
  const name = await api.createUserTheme(displayName, css);
  state.themes = [...state.themes, name].sort((a, b) => a.localeCompare(b, "es"));
  await update({ theme: `user:${name}` });
  return name;
}

async function duplicateBuiltin(id: string) {
  const theme = builtinThemes.find((t) => t.id === id);
  if (!theme) throw new Error("Tema desconocido");
  const header = `/* Copia de «${theme.name}». Edítalo a tu gusto: los cambios se ven al volver a la app. */\n`;
  return saveUserTheme(theme.name, header + theme.css);
}

/** Crea un tema desde cero con el editor de «Crear tema» (ver CreateThemeDialog.vue). */
async function createCustomTheme(displayName: string, css: string) {
  return saveUserTheme(displayName, css);
}

let listening = false;
function onFocus() {
  if (state.loaded) void reload().catch(() => undefined);
}

export function useAppearance() {
  if (!listening) {
    listening = true;
    window.addEventListener("focus", onFocus);
  }
  return {
    state: readonly(state),
    reload,
    setColorScheme: (colorScheme: ColorScheme) => update({ colorScheme }),
    setTheme: (theme: string | null) => update({ theme }),
    toggleSnippet: (name: string, on: boolean) =>
      update({ snippets: on ? [...state.settings.snippets.filter((s) => s !== name), name] : state.settings.snippets.filter((s) => s !== name) }),
    duplicateBuiltin,
    createCustomTheme,
    openFolder: (kind: CssKind) => api.openAppearanceFolder(kind),
    errorOf: (id: string) => state.errors[id],
  };
}
