<script setup lang="ts">
// Panel «Esquema»: los títulos de la nota con su jerarquía, como el índice de
// un lector de PDF. Clic en un título → la vista salta a él. Los títulos con
// subtítulos se pueden plegar.
import { computed, ref, watch } from "vue";
import { ChevronRight, X } from "lucide-vue-next";
import type { OutlineItem } from "../../lib/outline";

const props = defineProps<{
  items: OutlineItem[];
  /** Índice del título visible ahora mismo (resaltado). */
  activeIndex: number;
  /** Cambia al abrir otra nota: se reinician los plegados. */
  noteId: string;
}>();
const emit = defineEmits<{
  (e: "select", item: OutlineItem): void;
  (e: "close"): void;
}>();

/** Índices plegados. Por índice y no por posición: escribir encima no los mueve. */
const collapsed = ref(new Set<number>());
watch(
  () => props.noteId,
  () => (collapsed.value = new Set()),
);

// La sangría es relativa al nivel más alto presente: una nota que empieza en
// «##» no deja una columna vacía a la izquierda.
const baseLevel = computed(() => Math.min(...props.items.map((i) => i.level), 6));

const rows = computed(() => {
  const out: { item: OutlineItem; index: number; depth: number; hasChildren: boolean }[] = [];
  let hideBelow = Infinity; // nivel del título plegado que oculta a los siguientes
  props.items.forEach((item, index) => {
    if (item.level <= hideBelow) hideBelow = Infinity;
    if (hideBelow !== Infinity) return;
    const next = props.items[index + 1];
    const hasChildren = !!next && next.level > item.level;
    out.push({ item, index, depth: item.level - baseLevel.value, hasChildren });
    if (hasChildren && collapsed.value.has(index)) hideBelow = item.level;
  });
  return out;
});

function toggle(index: number) {
  const next = new Set(collapsed.value);
  if (!next.delete(index)) next.add(index);
  collapsed.value = next;
}

const allCollapsed = computed(() => rows.value.every((r) => !r.hasChildren || collapsed.value.has(r.index)));
function toggleAll() {
  if (allCollapsed.value) collapsed.value = new Set();
  else collapsed.value = new Set(props.items.map((item, i) => ((props.items[i + 1]?.level ?? 0) > item.level ? i : -1)).filter((i) => i >= 0));
}

const hasNesting = computed(() => props.items.some((item, i) => (props.items[i + 1]?.level ?? 0) > item.level));
</script>

<template>
  <aside class="note-outline" aria-label="Esquema de la nota">
    <header class="outline-header">
      <h2>Esquema</h2>
      <button v-if="hasNesting" type="button" class="outline-action" @click="toggleAll">
        {{ allCollapsed ? "Desplegar" : "Plegar" }}
      </button>
      <button type="button" class="icon-btn" aria-label="Ocultar esquema" title="Ocultar esquema (Ctrl+Mayús+O)" @click="emit('close')">
        <X :size="14" aria-hidden="true" />
      </button>
    </header>

    <p v-if="!items.length" class="outline-empty">
      Esta nota no tiene títulos. Empieza una línea con <code>#</code>, <code>##</code>, <code>###</code>… para crearlos.
    </p>

    <ul v-else class="outline-list">
      <li v-for="row in rows" :key="row.index" class="outline-row" :class="'outline-h' + row.item.level" :style="{ '--depth': row.depth }">
        <button
          v-if="row.hasChildren"
          type="button"
          class="outline-fold"
          :aria-expanded="!collapsed.has(row.index)"
          :aria-label="(collapsed.has(row.index) ? 'Desplegar ' : 'Plegar ') + row.item.text"
          @click="toggle(row.index)"
        >
          <ChevronRight :size="12" aria-hidden="true" />
        </button>
        <span v-else class="outline-fold-spacer" aria-hidden="true"></span>
        <button
          type="button"
          class="outline-link"
          :class="{ active: row.index === activeIndex }"
          :aria-current="row.index === activeIndex ? 'location' : undefined"
          :title="row.item.text"
          @click="emit('select', row.item)"
        >
          {{ row.item.text }}
        </button>
      </li>
    </ul>
  </aside>
</template>

<style>
.note-outline {
  width: 260px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-left: 1px solid var(--border);
  background: var(--surface);
  font-size: 13px;
}
.outline-header {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 8px 8px 14px;
  border-bottom: 1px solid var(--border);
}
.outline-header h2 {
  flex: 1;
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
}
.outline-action {
  font-size: 12px;
  color: var(--muted);
  padding: 2px 6px;
  border-radius: 4px;
}
.outline-action:hover {
  color: var(--fg);
  background: var(--surface-2);
}
.outline-empty {
  margin: 0;
  padding: 14px;
  color: var(--muted);
  line-height: 1.6;
}
.outline-empty code {
  font-family: var(--font-mono);
  font-size: 12px;
}
.outline-list {
  list-style: none;
  margin: 0;
  padding: 6px 6px 16px;
  overflow-y: auto;
  flex: 1;
}
.outline-row {
  display: flex;
  align-items: center;
  gap: 2px;
  padding-left: calc(var(--depth) * 14px);
}
.outline-fold,
.outline-fold-spacer {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.outline-fold {
  border-radius: 4px;
  color: var(--muted);
}
.outline-fold:hover {
  background: var(--surface-2);
  color: var(--fg);
}
.outline-fold svg {
  transition: transform 0.12s;
}
.outline-fold[aria-expanded="true"] svg {
  transform: rotate(90deg);
}
.outline-link {
  flex: 1;
  min-width: 0;
  text-align: left;
  padding: 4px 8px;
  border-radius: 5px;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.4;
}
.outline-link:hover {
  background: var(--surface-2);
  color: var(--fg);
}
.outline-link.active {
  background: var(--primary-soft);
  color: var(--fg);
}
.outline-h1 .outline-link {
  color: var(--fg);
  font-weight: 600;
}
.outline-h2 .outline-link {
  font-weight: 500;
}
@media (prefers-reduced-motion: reduce) {
  .outline-fold svg {
    transition: none;
  }
}
@container (max-width: 700px) {
  .note-outline {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    z-index: 10;
    box-shadow: var(--shadow);
    max-width: 85%;
  }
}
</style>
