<script setup lang="ts">
// Crear un tema desde cero, sin pasar por duplicar uno incluido y editar el
// CSS a mano (eso sigue existiendo, ver AppearanceDialog). El usuario solo
// elige los ~12 colores "esenciales" (ver customTheme.ts); el resto de
// variables que usan los temas incluidos se derivan para que el tema no
// quede roto, y el modo oscuro arranca de un borrador automático que puede
// editar antes de guardar.
import { computed, reactive, ref } from "vue";
import { AlertTriangle, Check, Wand2 } from "lucide-vue-next";
import Modal from "./Modal.vue";
import {
  DEFAULT_LIGHT,
  ESSENTIAL_FIELDS,
  buildThemeCss,
  contrastChecks,
  deriveDependents,
  invertForDark,
  isValidHex,
  type EssentialColors,
} from "../lib/customTheme";
import { useAppearance } from "../composables/useAppearance";
import { useToast } from "../composables/useToast";
import { ApiError } from "../lib/api";

const emit = defineEmits<{ (e: "close"): void }>();
const appearance = useAppearance();
const toast = useToast();

const name = ref("");
const mode = ref<"light" | "dark">("light");
const light = reactive<EssentialColors>({ ...DEFAULT_LIGHT });
const dark = reactive<EssentialColors>(invertForDark(DEFAULT_LIGHT));
const saving = ref(false);

const colors = computed(() => (mode.value === "light" ? light : dark));
const checks = computed(() => contrastChecks(colors.value));
const previewVars = computed(() => {
  const c = colors.value;
  const deps = deriveDependents(c, mode.value);
  return {
    "--bg": c.bg,
    "--surface": c.surface,
    "--surface-2": c.surfaceAlt,
    "--border": c.border,
    "--fg": c.fg,
    "--muted": c.muted,
    "--primary": c.primary,
    "--on-primary": deps.onPrimary,
    "--primary-soft": deps.primarySoft,
    "--link": c.link,
    "--success": c.success,
    "--warning": c.warning,
    "--danger": c.danger,
    "--focus": c.focus,
  };
});

function regenerateDark() {
  Object.assign(dark, invertForDark(light));
}

// Mientras se escribe el hex a mano puede pasar por valores incompletos
// («#12») que no bloquean la vista previa (se recalcula sola al corregirse),
// pero sí deben bloquear el guardado para no escribir un CSS roto.
const allValid = computed(() => [...Object.values(light), ...Object.values(dark)].every(isValidHex));

const message = (e: unknown) => (e instanceof ApiError ? e.message : e instanceof Error ? e.message : String(e));

async function save() {
  const trimmed = name.value.trim();
  if (!trimmed || !allValid.value) return;
  saving.value = true;
  try {
    const css = buildThemeCss(trimmed, light, dark);
    const saved = await appearance.createCustomTheme(trimmed, css);
    toast.success(`Tema «${saved}» creado y activado.`);
    emit("close");
  } catch (e) {
    toast.error(`No se pudo crear el tema: ${message(e)}`);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <Modal title="Crear tema personalizado" size="lg" @close="emit('close')">
    <p class="hint">
      Elige los colores clave (fondo, superficies, texto y acentos); el resto de variables que usa la app (sombra, resaltado de código, carpetas…) se calculan
      solas a partir de estos. Si luego quieres afinar algo a mano, el tema queda como un CSS normal en tu carpeta de temas.
    </p>

    <label class="field">
      <span>Nombre del tema</span>
      <input v-model="name" type="text" placeholder="Mi tema" maxlength="200" />
    </label>

    <div class="mode-switch" role="radiogroup" aria-label="Modo a editar">
      <label class="mode-tab">
        <input v-model="mode" type="radio" name="theme-edit-mode" class="sr-only" value="light" />
        Claro
      </label>
      <label class="mode-tab">
        <input v-model="mode" type="radio" name="theme-edit-mode" class="sr-only" value="dark" />
        Oscuro
      </label>
      <button v-if="mode === 'dark'" type="button" class="btn regen" @click="regenerateDark">
        <Wand2 :size="13" aria-hidden="true" />
        Regenerar desde el claro
      </button>
    </div>

    <div class="fields">
      <label v-for="f in ESSENTIAL_FIELDS" :key="f.key" class="field color-field">
        <span>{{ f.label }}</span>
        <span class="color-input">
          <input v-model="colors[f.key]" type="color" :aria-label="f.label" />
          <input v-model="colors[f.key]" type="text" class="hex" maxlength="7" :aria-label="`${f.label} (hex)`" />
        </span>
      </label>
    </div>

    <div class="preview" :style="previewVars">
      <p class="preview-label">Vista previa — {{ mode === "light" ? "Claro" : "Oscuro" }}</p>
      <div class="preview-card">
        <h4>Una nota de ejemplo</h4>
        <p>Texto normal y <span class="muted-text">un poco más apagado</span>, con un <a href="#">enlace</a> en medio.</p>
        <div class="preview-row">
          <button type="button" class="btn primary">Acción</button>
          <span class="pill success">Éxito</span>
          <span class="pill warning">Aviso</span>
          <span class="pill danger">Peligro</span>
        </div>
      </div>
    </div>

    <ul class="contrast-list">
      <li v-for="c in checks" :key="c.pair" :class="{ bad: !c.ok }">
        <Check v-if="c.ok" :size="14" aria-hidden="true" />
        <AlertTriangle v-else :size="14" aria-hidden="true" />
        {{ c.pair }}: {{ c.ratio }}:1 <span v-if="!c.ok">(mínimo {{ c.min }}:1)</span>
      </li>
    </ul>

    <template #footer>
      <button type="button" class="btn" :disabled="saving" @click="emit('close')">Cancelar</button>
      <button type="button" class="btn primary" :disabled="saving || !name.trim() || !allValid" @click="save">{{ saving ? "Creando…" : "Crear tema" }}</button>
    </template>
  </Modal>
</template>

<style scoped>
.hint {
  margin: 0 0 14px;
  color: var(--muted);
  font-size: 13px;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 13px;
  margin-bottom: 14px;
}
.field > input[type="text"] {
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--fg);
}
.mode-switch {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
}
.mode-tab {
  padding: 5px 12px;
  border: 1px solid var(--border);
  border-radius: 999px;
  font-size: 13px;
  cursor: pointer;
}
.mode-tab:has(input:checked) {
  background: var(--primary-soft);
  border-color: var(--primary);
  color: var(--fg);
}
.regen {
  margin-left: auto;
  font-size: 12px;
}
.fields {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px 14px;
  margin-bottom: 18px;
}
.color-field {
  margin-bottom: 0;
}
.color-input {
  display: flex;
  gap: 6px;
}
.color-input input[type="color"] {
  width: 32px;
  height: 32px;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: none;
  cursor: pointer;
}
.hex {
  flex: 1;
  min-width: 0;
  padding: 6px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--fg);
  font-family: var(--font-mono);
  font-size: 12px;
}
.preview {
  margin-bottom: 14px;
}
.preview-label {
  margin: 0 0 6px;
  font-size: 12px;
  color: var(--muted);
}
.preview-card {
  padding: 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg);
  color: var(--fg);
  box-shadow: var(--shadow);
}
.preview-card h4 {
  margin: 0 0 8px;
}
.preview-card p {
  margin: 0 0 12px;
}
.preview-card a {
  color: var(--link);
}
.muted-text {
  color: var(--muted);
}
.preview-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.preview-row .btn.primary {
  background: var(--primary);
  border-color: var(--primary);
  color: var(--on-primary);
}
.pill {
  padding: 3px 9px;
  border-radius: 999px;
  font-size: 12px;
  background: var(--surface-2);
}
.pill.success {
  color: var(--success);
}
.pill.warning {
  color: var(--warning);
}
.pill.danger {
  color: var(--danger);
}
.contrast-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
}
.contrast-list li {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--success);
}
.contrast-list li.bad {
  color: var(--danger);
}
</style>
