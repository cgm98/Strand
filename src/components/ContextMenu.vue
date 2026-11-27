<script setup lang="ts">
// Menú contextual único de la app. Se cierra al hacer clic fuera, al perder la
// ventana el foco, al desplazar o al redimensionar.
import { onBeforeUnmount, onMounted, ref } from "vue";
import MenuList from "./MenuList.vue";
import { useContextMenu } from "../composables/useContextMenu";

const { state, close } = useContextMenu();
const host = ref<HTMLElement>();

function onPointerDown(e: PointerEvent) {
  if (state.value && !host.value?.contains(e.target as Node)) close(false);
}
const closeQuietly = () => state.value && close(false);
const onScroll = (e: Event) => !host.value?.contains(e.target as Node) && closeQuietly();

onMounted(() => {
  document.addEventListener("pointerdown", onPointerDown, true);
  window.addEventListener("blur", closeQuietly);
  window.addEventListener("resize", closeQuietly);
  document.addEventListener("scroll", onScroll, true);
});
onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", onPointerDown, true);
  window.removeEventListener("blur", closeQuietly);
  window.removeEventListener("resize", closeQuietly);
  document.removeEventListener("scroll", onScroll, true);
});
</script>

<template>
  <div ref="host">
    <MenuList v-if="state" :key="`${state.x}:${state.y}`" :items="state.items" :x="state.x" :y="state.y" label="Menú contextual" @close="(r) => close(r)" />
  </div>
</template>
