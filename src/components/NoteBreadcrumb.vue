<script setup lang="ts">
// Dónde vive una nota: carpeta(s) + nombre, en una sola pieza reutilizable.
// Antes esto salía como texto suelto y distinto en cada sitio (el nav de
// migas en MarkdownEditor sin el nombre de la nota, el título y la ubicación
// como dos <span> separados en SearchPanel) — aquí se unifica con el mismo
// aspecto: icono de carpeta, migas apagadas y el nombre de la nota resaltado
// con el color de acento del tema, para que se note más que es lo importante.
import { computed } from "vue";
import { Folder } from "lucide-vue-next";

const props = withDefaults(defineProps<{ segments: string[]; name: string; size?: "sm" | "md" }>(), { size: "md" });

const crumbs = computed(() => (props.segments.length ? props.segments : ["Raíz"]));
const fullPath = computed(() => [...crumbs.value, props.name || "Sin título"].join(" / "));
</script>

<template>
  <nav class="note-breadcrumb" :class="`note-breadcrumb-${size}`" aria-label="Ubicación de la nota" :title="fullPath">
    <Folder :size="size === 'sm' ? 12 : 14" aria-hidden="true" class="note-breadcrumb-icon" />
    <template v-for="(segment, i) in crumbs" :key="i">
      <span class="note-breadcrumb-crumb">{{ segment }}</span>
      <span class="note-breadcrumb-sep" aria-hidden="true">›</span>
    </template>
    <span class="note-breadcrumb-name">{{ name || "Sin título" }}</span>
  </nav>
</template>

<style scoped>
.note-breadcrumb {
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
  overflow: hidden;
  line-height: 1.5;
}
.note-breadcrumb-icon {
  flex: none;
  color: var(--muted);
}
.note-breadcrumb-crumb {
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--muted);
}
.note-breadcrumb-sep {
  flex: none;
  color: var(--border);
}
.note-breadcrumb-name {
  flex: 1 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 650;
  color: var(--primary);
}
.note-breadcrumb-md {
  font-size: 13px;
}
.note-breadcrumb-sm {
  font-size: 12px;
}
</style>
