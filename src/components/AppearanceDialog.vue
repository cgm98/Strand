<script setup lang="ts">
// Apariencia: modo claro/oscuro, tema (incluido o propio) y fragmentos CSS.
// Los temas propios y los fragmentos son archivos que el usuario crea y edita
// en la carpeta de datos; aquí solo se eligen.
import { onMounted, ref } from "vue";
import { Copy, FolderOpen, Monitor, Moon, Plus, RefreshCw, Sun } from "lucide-vue-next";
import Modal from "./Modal.vue";
import CreateThemeDialog from "./CreateThemeDialog.vue";
import { ApiError, type ColorScheme, type CssKind } from "../lib/api";
import { builtinThemes, swatches, themeCategories } from "../themes";
import { useAppearance } from "../composables/useAppearance";
import { useToast } from "../composables/useToast";

const emit = defineEmits<{ (e: "close"): void }>();
const appearance = useAppearance();
const { state } = appearance;
const toast = useToast();
const reloading = ref(false);
const duplicating = ref("");
const creatingTheme = ref(false);

const message = (e: unknown) => (e instanceof ApiError ? e.message : e instanceof Error ? e.message : String(e));

async function run(task: () => Promise<unknown>, label: string) {
  try {
    await task();
  } catch (e) {
    toast.error(`${label}: ${message(e)}`);
  }
}

async function reload() {
  reloading.value = true;
  await run(appearance.reload, "No se pudo recargar");
  reloading.value = false;
}

const openFolder = (kind: CssKind) => run(() => appearance.openFolder(kind), "No se pudo abrir la carpeta");
const setTheme = (id: string | null) => run(() => appearance.setTheme(id), "No se pudo cambiar el tema");
const setScheme = (mode: ColorScheme) => run(() => appearance.setColorScheme(mode), "No se pudo cambiar el modo");

async function duplicate(id: string) {
  duplicating.value = id;
  try {
    const name = await appearance.duplicateBuiltin(id);
    toast.success(`Copiado como «${name}» en tus temas y activado. Pulsa «Abrir carpeta» para editarlo.`);
  } catch (e) {
    toast.error(`No se pudo duplicar: ${message(e)}`);
  } finally {
    duplicating.value = "";
  }
}

// Un tema guardado sin prefijo (versión anterior) es del usuario.
const isActive = (id: string | null) => {
  const current = state.settings.theme;
  if (!id || !current) return id === current;
  return current === id || (id.startsWith("user:") && current === id.slice(5));
};

const modes: Array<{ id: ColorScheme; label: string; icon: typeof Sun }> = [
  { id: "system", label: "Sistema", icon: Monitor },
  { id: "light", label: "Claro", icon: Sun },
  { id: "dark", label: "Oscuro", icon: Moon },
];

// Al abrir el diálogo, releer: el usuario puede haber añadido archivos.
onMounted(reload);
</script>

<template>
  <Modal title="Apariencia" size="md" @close="emit('close')">
    <section class="ap-section" aria-labelledby="ap-mode">
      <h3 id="ap-mode">Modo</h3>
      <div class="ap-segmented" role="radiogroup" aria-labelledby="ap-mode">
        <label v-for="mode in modes" :key="mode.id" class="ap-segment">
          <input type="radio" name="color-scheme" class="sr-only" :checked="state.settings.colorScheme === mode.id" @change="setScheme(mode.id)" />
          <component :is="mode.icon" :size="14" aria-hidden="true" />
          {{ mode.label }}
        </label>
      </div>
    </section>

    <section class="ap-section" aria-labelledby="ap-themes">
      <h3 id="ap-themes">Tema</h3>
      <div class="ap-options" role="radiogroup" aria-labelledby="ap-themes">
        <div class="ap-theme-grid ap-theme-grid-single">
          <div class="ap-theme-card">
            <label class="ap-theme-pick" title="Los colores propios de la app, sin ningún tema encima">
              <input type="radio" name="theme" class="sr-only" :checked="isActive(null)" @change="setTheme(null)" />
              <span class="ap-theme-preview ap-theme-preview-default" aria-hidden="true">Aa</span>
              <span class="ap-theme-name">Predeterminado</span>
            </label>
          </div>
        </div>

        <!-- Rejilla de tarjetas en vez de una fila por tema: con 5-6 temas incluidos
             (y más por venir, ver docs/design.md) una fila por tema agotaba el alto
             compartido del diálogo (modo + temas + fragmentos en un solo scroll). La
             descripción pasa a `title` (se ve al pasar el ratón) y a un `aria-label`
             propio en el input, para no perderla para lectores de pantalla. -->
        <template v-for="category in themeCategories" :key="category.id">
          <p class="ap-group">{{ category.label }}</p>
          <div class="ap-theme-grid">
            <div v-for="theme in builtinThemes.filter((t) => t.category === category.id)" :key="theme.id" class="ap-theme-card">
              <label class="ap-theme-pick" :title="theme.description">
                <input
                  type="radio"
                  name="theme"
                  class="sr-only"
                  :checked="isActive('builtin:' + theme.id)"
                  :aria-label="`${theme.name} — ${theme.description}`"
                  @change="setTheme('builtin:' + theme.id)"
                />
                <span class="ap-theme-preview" aria-hidden="true">
                  <span v-for="(color, i) in swatches(theme.css, state.scheme)" :key="i" :style="{ background: color }"></span>
                </span>
                <span class="ap-theme-name">{{ theme.name }}</span>
              </label>
              <button
                type="button"
                class="ap-theme-dup"
                :disabled="duplicating === theme.id"
                :aria-label="`Duplicar «${theme.name}» para editarlo`"
                title="Duplicar para editar"
                @click="duplicate(theme.id)"
              >
                <Copy :size="12" aria-hidden="true" />
              </button>
            </div>
          </div>
        </template>

        <div class="ap-group ap-group-row">
          <span>Tus temas</span>
          <span class="ap-group-actions">
            <button type="button" class="ap-link" @click="creatingTheme = true"><Plus :size="14" aria-hidden="true" /> Crear tema</button>
            <button type="button" class="ap-link" @click="openFolder('theme')"><FolderOpen :size="14" aria-hidden="true" /> Abrir carpeta</button>
          </span>
        </div>
        <p v-if="!state.themes.length" class="ap-empty">
          Aún no tienes temas propios. Duplica uno incluido con <Copy :size="12" aria-label="el botón de copiar" /> para empezar.
        </p>
        <label v-for="name in state.themes" :key="name" class="ap-option">
          <input type="radio" name="theme" :checked="isActive('user:' + name)" @change="setTheme('user:' + name)" />
          <span class="ap-text">
            <span class="ap-name">{{ name }}</span>
            <span v-if="appearance.errorOf('user:' + name) || appearance.errorOf(name)" class="ap-error">
              {{ appearance.errorOf("user:" + name) || appearance.errorOf(name) }}
            </span>
          </span>
        </label>
      </div>
    </section>

    <section class="ap-section" aria-labelledby="ap-snippets">
      <div class="ap-group-row">
        <h3 id="ap-snippets">Fragmentos CSS</h3>
        <button type="button" class="ap-link" @click="openFolder('snippet')"><FolderOpen :size="14" aria-hidden="true" /> Abrir carpeta</button>
      </div>
      <p class="ap-help">Pequeños ajustes que se aplican encima del tema. Activa los que quieras.</p>
      <p v-if="!state.snippets.length" class="ap-empty">No hay fragmentos. Añade archivos <code>.css</code> a la carpeta y pulsa «Recargar».</p>
      <ul v-else class="ap-options">
        <li v-for="name in state.snippets" :key="name">
          <label class="ap-option">
            <input
              type="checkbox"
              role="switch"
              :checked="state.settings.snippets.includes(name)"
              @change="run(() => appearance.toggleSnippet(name, ($event.target as HTMLInputElement).checked), 'No se pudo cambiar el fragmento')"
            />
            <span class="ap-text">
              <span class="ap-name">{{ name }}</span>
              <span v-if="appearance.errorOf('snippet:' + name)" class="ap-error">{{ appearance.errorOf("snippet:" + name) }}</span>
            </span>
          </label>
        </li>
      </ul>
    </section>

    <template #footer>
      <p class="ap-footer-hint">Los cambios en los archivos se aplican al volver a la ventana.</p>
      <button type="button" class="btn" :disabled="reloading" @click="reload">
        <RefreshCw :size="14" aria-hidden="true" /> {{ reloading ? "Recargando…" : "Recargar" }}
      </button>
    </template>
  </Modal>
  <CreateThemeDialog v-if="creatingTheme" @close="creatingTheme = false" />
</template>

<style scoped>
.ap-section + .ap-section {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}
.ap-section h3 {
  margin: 0 0 8px;
  font-size: 14px;
}
.ap-group-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.ap-group-row h3 {
  margin: 0;
}
.ap-group {
  margin: 12px 0 4px;
  padding: 0 10px;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}
.ap-group.ap-group-row {
  padding-right: 0;
}
.ap-group-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}
.ap-link {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 400;
  text-transform: none;
  letter-spacing: 0;
  color: var(--link);
  padding: 2px 4px;
  border-radius: 4px;
}
.ap-link:hover {
  text-decoration: underline;
}
.ap-help {
  margin: 4px 0 10px;
  color: var(--muted);
  font-size: 13px;
}
.ap-empty {
  margin: 0;
  padding: 10px 12px;
  border: 1px dashed var(--border);
  border-radius: 6px;
  color: var(--muted);
  font-size: 13px;
}
.ap-empty svg {
  vertical-align: -2px;
}

/* Modo: control segmentado */
.ap-segmented {
  display: inline-flex;
  padding: 3px;
  gap: 2px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
}
.ap-segment {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 12px;
  border-radius: 6px;
  font-size: 13px;
  color: var(--muted);
  cursor: pointer;
}
.ap-segment:hover {
  color: var(--fg);
}
.ap-segment:has(input:checked) {
  background: var(--bg);
  color: var(--fg);
  box-shadow: 0 1px 3px rgb(0 0 0 / 0.15);
}
.ap-segment:has(input:focus-visible) {
  outline: 2px solid var(--focus);
  outline-offset: 1px;
}

/* Lista de opciones */
.ap-options {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 2px;
}
.ap-option {
  display: flex;
  align-items: center;
  gap: 10px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
  padding: 7px 10px;
}
.ap-option:hover {
  background: var(--surface);
}
.ap-option:has(input:checked) {
  background: var(--primary-soft);
}
.ap-option input {
  accent-color: var(--primary);
  margin: 0;
  flex-shrink: 0;
}
.ap-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.ap-name {
  overflow-wrap: anywhere;
}
.ap-error {
  color: var(--danger);
  font-size: 12px;
}

/* Rejilla de temas incluidos: tarjetas compactas en vez de una fila por tema
   (ver comentario en la plantilla). `minmax` hace que quepan tantas como el
   ancho del diálogo permita y se reparta el resto; crece en filas, no en alto
   acumulado por tema. */
.ap-theme-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
  gap: 8px;
  margin: 0 0 4px;
}
.ap-theme-grid-single {
  grid-template-columns: 96px;
}
.ap-theme-card {
  position: relative;
}
.ap-theme-pick {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 6px;
  border: 1px solid var(--border);
  border-radius: 8px;
  cursor: pointer;
}
.ap-theme-pick:hover {
  border-color: var(--primary);
}
.ap-theme-pick:has(input:checked) {
  border-color: var(--primary);
  background: var(--primary-soft);
}
.ap-theme-pick:has(input:focus-visible) {
  outline: 2px solid var(--focus);
  outline-offset: 2px;
}
.ap-theme-preview {
  display: flex;
  height: 38px;
  border-radius: 5px;
  overflow: hidden;
}
.ap-theme-preview span {
  flex: 1;
}
/* Con 8 franjas finas (antes 5) el límite entre dos colores parecidos (p. ej.
   superficie y superficie elevada) se perdía; un filete fijo, independiente
   del tema, marca el corte sin tener que calibrarlo color a color. */
.ap-theme-preview span:not(:last-child) {
  box-shadow: inset -1px 0 0 rgb(0 0 0 / 0.12);
}
.ap-theme-preview-default {
  align-items: center;
  justify-content: center;
  background: var(--surface);
  color: var(--muted);
  font-size: 13px;
  font-weight: 600;
}
.ap-theme-name {
  font-size: 12px;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
/* Siempre visible, nunca solo al pasar el ratón (igual que las acciones de los
   bloques de código/fórmula/mermaid: ver docs/design.md). */
.ap-theme-dup {
  position: absolute;
  top: 4px;
  right: 4px;
  display: inline-flex;
  padding: 4px;
  border-radius: 4px;
  color: var(--muted);
  background: color-mix(in srgb, var(--surface) 70%, transparent);
}
.ap-theme-dup:hover:not(:disabled) {
  color: var(--fg);
  background: var(--surface-2);
}
.ap-theme-dup:disabled {
  opacity: 0.5;
}

.ap-footer-hint {
  margin: 0 auto 0 0;
  color: var(--muted);
  font-size: 12px;
}
.btn svg {
  vertical-align: -2px;
}
code {
  font-family: var(--font-mono);
  font-size: 12px;
}
</style>
