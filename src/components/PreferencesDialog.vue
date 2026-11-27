<script setup lang="ts">
// Preferencias generales (no de apariencia: eso tiene su propio diálogo).
// Hoy solo trae el intervalo de autoguardado; es el lugar donde sumar las
// próximas (ver hito 8 en MIGRACION.md).
import { AUTOSAVE_DELAYS, usePreferences } from "../composables/usePreferences";
import Modal from "./Modal.vue";

const emit = defineEmits<{ (e: "close"): void }>();
const preferences = usePreferences();
</script>

<template>
  <Modal title="Preferencias" size="sm" @close="emit('close')">
    <section class="pref-section" aria-labelledby="pref-autosave">
      <h3 id="pref-autosave">Autoguardado</h3>
      <p class="pref-help">Cuánto esperar después de dejar de escribir antes de guardar.</p>
      <div class="pref-options" role="radiogroup" aria-labelledby="pref-autosave">
        <label v-for="delay in AUTOSAVE_DELAYS" :key="delay.ms" class="pref-option">
          <input
            type="radio"
            name="autosave-delay"
            :checked="preferences.state.autosaveDelayMs === delay.ms"
            @change="preferences.setAutosaveDelay(delay.ms)"
          />
          {{ delay.label }}
        </label>
      </div>
    </section>
  </Modal>
</template>

<style scoped>
.pref-section h3 {
  margin: 0 0 4px;
  font-size: 14px;
}
.pref-help {
  margin: 0 0 10px;
  color: var(--muted);
  font-size: 13px;
}
.pref-options {
  display: grid;
  gap: 2px;
}
.pref-option {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 10px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
}
.pref-option:hover {
  background: var(--surface);
}
.pref-option:has(input:checked) {
  background: var(--primary-soft);
}
.pref-option input {
  accent-color: var(--primary);
  margin: 0;
  flex-shrink: 0;
}
</style>
