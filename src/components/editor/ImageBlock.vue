<script setup lang="ts">
// Contenedor de una imagen dentro del editor: muestra la imagen y su enlace, y
// agrupa las acciones ampliar, editar (texto alternativo, enlace o archivo),
// añadir imágenes detrás y eliminar. Lo monta `imageWidget.ts` en CodeMirror;
// los cambios vuelven al documento como Markdown, así que Ctrl+Z los deshace.
import { computed, nextTick, ref, watch } from "vue";
import { Code2, ImagePlus, Maximize2, Pencil, Trash2 } from "lucide-vue-next";
import ImageLightbox from "../ImageLightbox.vue";
import { imageUrl, parseImage, pickImageFiles, serializeImage, uploadImages } from "../../lib/images";
import { useToast } from "../../composables/useToast";
import { useContextMenu } from "../../composables/useContextMenu";

const props = defineProps<{ source: string; editingSource: boolean }>();
const emit = defineEmits<{
  (e: "update", markdown: string): void;
  (e: "remove"): void;
  (e: "insertAfter", markdowns: string[]): void;
  (e: "editSource"): void;
}>();
const toast = useToast();
const menu = useContextMenu();

const image = computed(() => parseImage(props.source));
const url = computed(() => (image.value ? imageUrl(image.value.src) : null));
const broken = ref(false);
watch(url, () => (broken.value = false));

const zoomed = ref(false);
const editing = ref(false);
const busy = ref(false);
const altDraft = ref("");
const srcDraft = ref("");
const altInput = ref<HTMLInputElement>();

async function startEdit() {
  if (!image.value) return;
  altDraft.value = image.value.alt;
  srcDraft.value = image.value.src;
  editing.value = true;
  await nextTick();
  altInput.value?.focus();
}

function saveEdit() {
  if (!image.value || !srcDraft.value.trim()) return;
  emit("update", serializeImage({ ...image.value, alt: altDraft.value, src: srcDraft.value }));
  editing.value = false;
}

async function replaceFile() {
  const [file] = await pickImageFiles(false);
  if (!file) return;
  busy.value = true;
  const [uploaded] = await uploadImages([file], toast.error);
  busy.value = false;
  if (uploaded) srcDraft.value = uploaded.src;
}

async function addImages() {
  const files = await pickImageFiles(true);
  if (!files.length) return;
  busy.value = true;
  const refs = await uploadImages(files, toast.error);
  busy.value = false;
  if (refs.length) emit("insertAfter", refs.map(serializeImage));
}

function openMenu(event: MouseEvent) {
  menu.open(event, [
    { label: "Ampliar", disabled: !url.value || broken.value, action: () => (zoomed.value = true) },
    { label: "Editar…", action: startEdit },
    { label: "Añadir imágenes debajo…", disabled: busy.value, action: addImages },
    { label: "Ver Markdown", disabled: props.editingSource, action: () => emit("editSource") },
    { label: "Copiar enlace", action: () => navigator.clipboard.writeText(image.value?.src ?? "") },
    "separator",
    { label: "Eliminar", danger: true, action: remove },
  ]);
}

function remove() {
  emit("remove");
  toast.info("Imagen quitada de la nota. Ctrl+Z la recupera.");
}
</script>

<template>
  <div
    class="img-card"
    :class="{ 'img-card-source': editingSource }"
    role="group"
    :aria-label="`Imagen: ${image?.alt || 'sin descripción'}`"
    @contextmenu="openMenu"
  >
    <button v-if="url && !broken" type="button" class="img-view" :title="'Ampliar'" :aria-label="`Ampliar imagen ${image?.alt ?? ''}`" @click="zoomed = true">
      <img :src="url" :alt="image?.alt ?? ''" draggable="false" @error="broken = true" />
    </button>
    <div v-else class="img-missing" role="note">
      {{ url ? "No se encontró el archivo de la imagen." : "Este enlace de imagen no se puede mostrar." }}
    </div>

    <div class="img-meta">
      <span class="img-alt" :class="{ empty: !image?.alt }">{{ image?.alt || "Sin texto alternativo" }}</span>
      <code class="img-src" :title="image?.src">{{ image?.src }}</code>
    </div>

    <div class="img-actions" role="toolbar" aria-label="Acciones de la imagen">
      <button type="button" :disabled="!url || broken" title="Ampliar" @click="zoomed = true"><Maximize2 :size="14" aria-hidden="true" />Ampliar</button>
      <button type="button" :aria-expanded="editing" title="Editar texto alternativo, enlace o archivo" @click="editing ? (editing = false) : startEdit()">
        <Pencil :size="14" aria-hidden="true" />Editar
      </button>
      <button type="button" :disabled="busy" title="Añadir imágenes debajo de esta" @click="addImages">
        <ImagePlus :size="14" aria-hidden="true" />{{ busy ? "Subiendo…" : "Añadir" }}
      </button>
      <button v-if="!editingSource" type="button" title="Editar el Markdown de la imagen" @click="emit('editSource')">
        <Code2 :size="14" aria-hidden="true" />Markdown
      </button>
      <button type="button" class="danger" title="Quitar la imagen de la nota" @click="remove"><Trash2 :size="14" aria-hidden="true" />Eliminar</button>
    </div>

    <form v-if="editing" class="img-form" @submit.prevent="saveEdit" @keydown.esc.stop="editing = false">
      <label>
        Texto alternativo
        <input ref="altInput" v-model="altDraft" class="field" placeholder="Describe la imagen para quien no pueda verla" />
      </label>
      <label>
        Enlace
        <input v-model="srcDraft" class="field mono" required placeholder="attachments/… o https://…" />
      </label>
      <div class="img-form-actions">
        <button type="button" class="btn" :disabled="busy" @click="replaceFile">{{ busy ? "Subiendo…" : "Cambiar archivo…" }}</button>
        <span class="spacer"></span>
        <button type="button" class="btn" @click="editing = false">Cancelar</button>
        <button type="submit" class="btn primary" :disabled="busy || !srcDraft.trim()">Guardar</button>
      </div>
    </form>

    <ImageLightbox v-if="zoomed && url" :src="url" :alt="image?.alt ?? ''" @close="zoomed = false" />
  </div>
</template>

<style>
.cm-image-host {
  display: inline-block;
  max-width: 100%;
  vertical-align: top;
  white-space: normal;
}
.cm-image-host.editing-source {
  display: block;
  margin: 6px 0 4px;
}
.img-card {
  display: inline-flex;
  flex-direction: column;
  max-width: 100%;
  margin: 6px 0;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--surface);
  overflow: hidden;
  font-family: var(--font-sans);
  font-size: 13px;
  line-height: 1.4;
  cursor: default;
}
.img-card:focus-within {
  border-color: var(--focus);
}
.img-view {
  display: block;
  padding: 0;
  background: var(--bg);
  cursor: zoom-in;
}
.img-view img {
  display: block;
  max-width: 100%;
  max-height: 60vh;
  object-fit: contain;
  margin: 0 auto;
}
.img-missing {
  padding: 24px 16px;
  color: var(--muted);
  text-align: center;
  background: var(--bg);
}
.img-meta {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 12px 0;
  min-width: 0;
}
.img-alt {
  font-weight: 600;
  overflow-wrap: anywhere;
}
.img-alt.empty {
  color: var(--warning);
  font-weight: 500;
}
.img-src {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.img-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
  padding: 6px 8px 8px;
}
.img-actions button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 8px;
  border-radius: 5px;
  color: var(--muted);
  font-size: 12px;
}
.img-actions button:hover:not(:disabled) {
  color: var(--fg);
  background: var(--surface-2);
}
.img-actions button.danger:hover {
  color: var(--danger);
}
.img-actions button[aria-expanded="true"] {
  color: var(--fg);
  background: var(--surface-2);
}
.img-form {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px 12px;
  border-top: 1px solid var(--border);
}
.img-form label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  color: var(--muted);
}
.img-form .field {
  color: var(--fg);
  font-size: 13px;
}
.img-form .mono {
  font-family: var(--font-mono);
  font-size: 12px;
}
.img-form-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.img-form-actions .spacer {
  flex: 1;
}
</style>
