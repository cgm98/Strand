<script setup lang="ts">
// Sustituye a la búsqueda híbrida de cogni-vault (FTS5 + sqlite-vec) por la
// búsqueda de texto completo FTS5 del backend: título ×5, sin tildes, por prefijo.
import { nextTick, onMounted, ref, watch } from "vue";
import Modal from "./Modal.vue";
import NoteBreadcrumb from "./NoteBreadcrumb.vue";
import { api, ApiError, highlight, type SearchHit } from "../lib/api";
import { folderPath } from "../lib/noteTree";
import type { FolderItem } from "../types";

const props = defineProps<{ folders: FolderItem[]; initialQuery?: string }>();
const emit = defineEmits<{ (e: "close"): void; (e: "selectFile", id: string): void }>();

const query = ref(props.initialQuery ?? "");
const hits = ref<SearchHit[]>([]);
const loading = ref(false);
const error = ref("");
const activeIndex = ref(0);
const input = ref<HTMLInputElement>();
let timer: ReturnType<typeof setTimeout> | undefined;
let request = 0;

async function run(q: string) {
  const current = ++request;
  if (!q.trim()) {
    hits.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  error.value = "";
  try {
    const result = await api.searchNotes(q, 50);
    if (current !== request) return;
    hits.value = result;
    activeIndex.value = 0;
  } catch (e) {
    if (current === request) error.value = e instanceof ApiError ? e.message : String(e);
  } finally {
    if (current === request) loading.value = false;
  }
}

watch(query, (q) => {
  clearTimeout(timer);
  timer = setTimeout(() => void run(q), 180);
});

function choose(hit: SearchHit | undefined) {
  if (!hit) return;
  emit("selectFile", hit.id);
  emit("close");
}

function onKey(e: KeyboardEvent) {
  const n = hits.value.length;
  if ((e.key === "ArrowDown" || e.key === "ArrowUp") && n) {
    e.preventDefault();
    activeIndex.value = (activeIndex.value + (e.key === "ArrowDown" ? 1 : -1) + n) % n;
    void nextTick(() => document.getElementById(`hit-${activeIndex.value}`)?.scrollIntoView({ block: "nearest" }));
  } else if (e.key === "Enter") {
    e.preventDefault();
    choose(hits.value[activeIndex.value]);
  }
}

const segmentsOf = (hit: SearchHit) => folderPath(props.folders, hit.folderId);

onMounted(() => {
  input.value?.focus();
  input.value?.select();
  if (query.value) void run(query.value);
});
</script>

<template>
  <Modal title="Buscar en las notas" size="lg" placement="top" @close="emit('close')">
    <input
      ref="input"
      v-model="query"
      class="field search-input"
      type="search"
      role="combobox"
      aria-label="Texto a buscar"
      aria-controls="search-results"
      aria-expanded="true"
      aria-autocomplete="list"
      :aria-activedescendant="hits.length ? `hit-${activeIndex}` : undefined"
      placeholder="Palabras del título o del contenido…"
      @keydown="onKey"
    />
    <p class="search-status" role="status">
      <template v-if="loading">Buscando…</template>
      <template v-else-if="error">{{ error }}</template>
      <template v-else-if="query.trim()">{{ hits.length }} resultado{{ hits.length === 1 ? "" : "s" }}</template>
      <template v-else>Busca sin preocuparte por tildes ni mayúsculas; «tau» encuentra «Tauri».</template>
    </p>
    <ul id="search-results" class="hits" role="listbox" aria-label="Resultados">
      <li
        v-for="(hit, i) in hits"
        :id="`hit-${i}`"
        :key="hit.id"
        role="option"
        class="hit"
        :aria-selected="i === activeIndex"
        @click="choose(hit)"
        @mousemove="activeIndex = i"
      >
        <NoteBreadcrumb class="hit-location" :segments="segmentsOf(hit)" :name="hit.title" size="sm" />
        <span class="hit-snippet" v-html="highlight(hit.snippet)"></span>
      </li>
    </ul>
  </Modal>
</template>

<style scoped>
.search-input {
  min-height: 2.4rem;
  font-size: 15px;
}
.search-status {
  margin: 8px 2px;
  font-size: 12px;
  color: var(--muted);
}
.hits {
  list-style: none;
  margin: 0;
  padding: 0;
  max-height: 55vh;
  overflow: auto;
}
.hit {
  display: grid;
  gap: 4px;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
}
.hit[aria-selected="true"] {
  background: var(--primary-soft);
}
.hit-location {
  min-width: 0;
}
.hit-snippet {
  font-size: 13px;
  color: var(--muted);
}
.hit-snippet :deep(mark) {
  background: var(--mark);
  color: var(--fg);
  border-radius: 2px;
  padding: 0 1px;
}
</style>
