<script setup lang="ts">
// Lista de un menú contextual (recursiva para los submenús). Patrón ARIA "menu":
// flechas para moverse, → abre submenú, ← vuelve, Intro/Espacio activa, Esc cierra.
import { computed, nextTick, onMounted, ref } from "vue";
import { Check, ChevronRight } from "lucide-vue-next";
import type { MenuEntry, MenuItem } from "../composables/useContextMenu";

const props = defineProps<{ items: MenuEntry[]; x: number; y: number; depth?: number; label?: string }>();
const emit = defineEmits<{ (e: "close", restoreFocus: boolean): void; (e: "back"): void }>();

const root = ref<HTMLElement>();
const pos = ref({ left: props.x, top: props.y });
const openIndex = ref<number | null>(null);
const subAnchor = ref({ x: 0, y: 0, left: 0 });
const buttons = ref<HTMLButtonElement[]>([]);
const sub = ref<{ focusFirst: () => void } | null>(null);

const isItem = (e: MenuEntry): e is MenuItem => e !== "separator";
const enabledIndexes = computed(() => props.items.flatMap((e, i) => (isItem(e) && !e.disabled ? [i] : [])));

onMounted(async () => {
  // Que el menú no se salga de la ventana; un submenú se abre hacia la izquierda si no cabe.
  const rect = root.value!.getBoundingClientRect();
  const margin = 8;
  let left = props.x;
  if (left + rect.width > window.innerWidth - margin) left = props.depth ? subAnchor.value.left - rect.width : window.innerWidth - rect.width - margin;
  const top = Math.min(props.y, window.innerHeight - rect.height - margin);
  pos.value = { left: Math.max(margin, left), top: Math.max(margin, top) };
  if (!props.depth) {
    await nextTick();
    focusFirst();
  }
});

function focusAt(index: number) {
  buttons.value.find((b) => Number(b.dataset.index) === index)?.focus();
}
function focusFirst() {
  if (enabledIndexes.value.length) focusAt(enabledIndexes.value[0]);
}
defineExpose({ focusFirst });

function currentIndex() {
  return Number((document.activeElement as HTMLElement | null)?.dataset.index ?? -1);
}

async function openSub(index: number, focus: boolean) {
  const button = buttons.value.find((b) => Number(b.dataset.index) === index);
  if (!button) return;
  const rect = button.getBoundingClientRect();
  subAnchor.value = { x: rect.right - 4, y: rect.top - 5, left: rect.left + 4 };
  openIndex.value = index;
  if (focus) {
    await nextTick();
    sub.value?.focusFirst();
  }
}

function activate(index: number) {
  const item = props.items[index];
  if (!isItem(item) || item.disabled) return;
  if (item.children) return void openSub(index, true);
  // Cerrar primero (devuelve el foco al editor) y luego ejecutar la acción.
  emit("close", true);
  setTimeout(() => item.action?.(), 0);
}

function onKey(e: KeyboardEvent) {
  const list = enabledIndexes.value;
  const at = list.indexOf(currentIndex());
  const move = (to: number) => {
    e.preventDefault();
    focusAt(list[(to + list.length) % list.length]);
  };
  switch (e.key) {
    case "ArrowDown":
      return move(at + 1);
    case "ArrowUp":
      return move(at < 0 ? list.length - 1 : at - 1);
    case "Home":
      return move(0);
    case "End":
      return move(list.length - 1);
    case "ArrowRight": {
      const item = props.items[currentIndex()];
      if (isItem(item) && item.children) {
        e.preventDefault();
        void openSub(currentIndex(), true);
      }
      return;
    }
    case "ArrowLeft":
      if (props.depth) {
        e.preventDefault();
        emit("back");
      }
      return;
    case "Escape":
      e.preventDefault();
      return emit("close", true);
    case "Tab":
      e.preventDefault();
      return emit("close", true);
    case "Enter":
    case " ":
      e.preventDefault();
      return activate(currentIndex());
  }
}

function onHover(index: number) {
  const item = props.items[index];
  if (isItem(item) && item.children && !item.disabled) void openSub(index, false);
  else openIndex.value = null;
  focusAt(index);
}

async function closeSub() {
  const index = openIndex.value;
  openIndex.value = null;
  await nextTick();
  if (index !== null) focusAt(index);
}

const openItem = computed(() => (openIndex.value !== null ? (props.items[openIndex.value] as MenuItem) : null));
</script>

<template>
  <div
    ref="root"
    class="ctx-menu"
    role="menu"
    :aria-label="label"
    :style="{ left: pos.left + 'px', top: pos.top + 'px' }"
    @keydown.stop="onKey"
    @contextmenu.prevent
  >
    <template v-for="(entry, i) in items" :key="i">
      <div v-if="entry === 'separator'" class="ctx-separator" role="separator"></div>
      <button
        v-else
        ref="buttons"
        type="button"
        class="ctx-item"
        :class="{ danger: entry.danger, open: openIndex === i }"
        :role="entry.checked !== undefined ? 'menuitemcheckbox' : 'menuitem'"
        :aria-checked="entry.checked !== undefined ? entry.checked : undefined"
        :aria-haspopup="entry.children ? 'menu' : undefined"
        :aria-expanded="entry.children ? openIndex === i : undefined"
        :disabled="entry.disabled"
        :data-index="i"
        tabindex="-1"
        @click="activate(i)"
        @mouseenter="onHover(i)"
      >
        <span class="ctx-check" aria-hidden="true"><Check v-if="entry.checked" :size="14" /></span>
        <span class="ctx-label">{{ entry.label }}</span>
        <span v-if="entry.shortcut" class="ctx-shortcut">{{ entry.shortcut }}</span>
        <ChevronRight v-if="entry.children" :size="14" class="ctx-arrow" aria-hidden="true" />
      </button>
    </template>
    <MenuList
      v-if="openItem?.children"
      ref="sub"
      :items="openItem.children"
      :x="subAnchor.x"
      :y="subAnchor.y"
      :depth="(depth ?? 0) + 1"
      :label="openItem.label"
      @close="(r) => emit('close', r)"
      @back="closeSub"
    />
  </div>
</template>

<style>
.ctx-menu {
  position: fixed;
  z-index: 10000;
  min-width: 220px;
  max-width: 320px;
  max-height: calc(100vh - 16px);
  overflow-y: auto;
  padding: 5px;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: var(--shadow);
  font-size: 13px;
}
.ctx-item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  min-height: 30px;
  padding: 4px 8px 4px 4px;
  border-radius: 5px;
  text-align: left;
  color: var(--fg);
}
.ctx-item:focus,
.ctx-item.open {
  outline: none;
  background: var(--surface-2);
}
.ctx-item:focus-visible {
  box-shadow: inset 0 0 0 2px var(--focus);
}
.ctx-item:disabled {
  color: var(--muted);
  opacity: 0.6;
}
.ctx-item.danger {
  color: var(--danger);
}
.ctx-check {
  width: 16px;
  display: inline-flex;
  justify-content: center;
  color: var(--primary);
  flex-shrink: 0;
}
.ctx-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ctx-shortcut {
  color: var(--muted);
  font-size: 11px;
  font-family: var(--font-mono);
}
.ctx-arrow {
  color: var(--muted);
}
.ctx-separator {
  height: 1px;
  margin: 4px 6px;
  background: var(--border);
}
</style>
