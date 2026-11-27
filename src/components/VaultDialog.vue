<script setup lang="ts">
// Copia de seguridad: exportar a zip, importar zip (propio, cogni-vault u
// Obsidian) y migrar una base cogni_vault.db. Sustituye al BackupModal de cogni-vault.
import { ref } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import Modal from "./Modal.vue";
import { api, ApiError } from "../lib/api";
import { useToast } from "../composables/useToast";
import { useConfirm } from "../composables/useConfirm";

const emit = defineEmits<{ (e: "close"): void; (e: "changed"): void }>();
const toast = useToast();
const { confirm } = useConfirm();
const includeTrash = ref(false);
const busy = ref<"" | "export" | "import" | "legacy" | "prune">("");

const fail = (label: string, e: unknown) => toast.error(`${label}: ${e instanceof ApiError ? e.message : String(e)}`);

async function exportZip() {
  const date = new Date().toISOString().slice(0, 10);
  const dest = await save({ defaultPath: `notas-${date}.zip`, filters: [{ name: "Zip", extensions: ["zip"] }] });
  if (!dest) return;
  busy.value = "export";
  try {
    const r = await api.exportVault(dest, includeTrash.value);
    toast.success(`Exportadas ${r.notes} notas, ${r.folders} carpetas y ${r.attachments} imágenes`);
  } catch (e) {
    fail("No se pudo exportar", e);
  } finally {
    busy.value = "";
  }
}

async function importZip() {
  const src = await open({ filters: [{ name: "Zip", extensions: ["zip"] }] });
  if (!src) return;
  busy.value = "import";
  try {
    const r = await api.importVault(src);
    toast.success(`Importadas ${r.notesCreated} notas (${r.notesSkipped} ya existían) y ${r.attachmentsImported} imágenes`);
    if (r.attachmentsMissing) toast.warning(`Faltan ${r.attachmentsMissing} imágenes referenciadas en el zip`);
    emit("changed");
  } catch (e) {
    fail("No se pudo importar", e);
  } finally {
    busy.value = "";
  }
}

async function importLegacy() {
  const src = await open({ filters: [{ name: "Base de datos SQLite", extensions: ["db", "sqlite", "sqlite3"] }] });
  if (!src) return;
  busy.value = "legacy";
  try {
    const r = await api.importCogniVaultDb(src);
    toast.success(`Migradas ${r.notesCreated} notas, ${r.foldersCreated} carpetas y ${r.versionsImported} versiones`);
    if (r.attachmentsMissing) toast.warning(`No se encontraron ${r.attachmentsMissing} imágenes junto a la base`);
    emit("changed");
  } catch (e) {
    fail("No se pudo migrar", e);
  } finally {
    busy.value = "";
  }
}

async function pruneOrphans() {
  const ok = await confirm({
    title: "Borrar adjuntos huérfanos",
    message: "Se borrarán para siempre las imágenes de attachments/ que ninguna nota ni versión guardada use. No se puede deshacer.",
    confirmLabel: "Borrar huérfanos",
    destructive: true,
  });
  if (!ok) return;
  busy.value = "prune";
  try {
    const r = await api.pruneOrphanAttachments();
    toast.success(r.removed.length ? `Borradas ${r.removed.length} imagen(es) huérfana(s)` : "No había imágenes huérfanas");
  } catch (e) {
    fail("No se pudo limpiar", e);
  } finally {
    busy.value = "";
  }
}
</script>

<template>
  <Modal title="Copia de seguridad" size="md" @close="emit('close')">
    <section class="vault-section" aria-labelledby="v-export">
      <h3 id="v-export">Exportar</h3>
      <p>Guarda todas las notas como archivos Markdown en un zip, con las imágenes incluidas. Se abre en cualquier editor.</p>
      <label class="check"><input v-model="includeTrash" type="checkbox" /> Incluir la papelera</label>
      <button type="button" class="btn primary" :disabled="!!busy" @click="exportZip">{{ busy === "export" ? "Exportando…" : "Exportar a zip…" }}</button>
    </section>
    <section class="vault-section" aria-labelledby="v-import">
      <h3 id="v-import">Importar</h3>
      <p>Admite zips de esta app, de cogni-vault y de Obsidian. Las notas que ya existen no se duplican.</p>
      <button type="button" class="btn" :disabled="!!busy" @click="importZip">{{ busy === "import" ? "Importando…" : "Importar zip…" }}</button>
    </section>
    <section class="vault-section" aria-labelledby="v-legacy">
      <h3 id="v-legacy">Migrar desde cogni-vault</h3>
      <p>Lee <code>cogni_vault.db</code> sin modificarlo. Los espacios y la taxonomía pasan a ser carpetas; las imágenes se buscan junto a la base.</p>
      <button type="button" class="btn" :disabled="!!busy" @click="importLegacy">{{ busy === "legacy" ? "Migrando…" : "Elegir cogni_vault.db…" }}</button>
    </section>
    <section class="vault-section" aria-labelledby="v-prune">
      <h3 id="v-prune">Limpieza</h3>
      <p>Borra las imágenes de <code>attachments/</code> que ya no usa ninguna nota ni versión guardada.</p>
      <button type="button" class="btn danger" :disabled="!!busy" @click="pruneOrphans">
        {{ busy === "prune" ? "Borrando…" : "Borrar adjuntos huérfanos…" }}
      </button>
    </section>
  </Modal>
</template>

<style scoped>
.vault-section + .vault-section {
  margin-top: 20px;
  padding-top: 16px;
  border-top: 1px solid var(--border);
}
.vault-section h3 {
  margin: 0 0 4px;
  font-size: 14px;
}
.vault-section p {
  margin: 0 0 10px;
  color: var(--muted);
  font-size: 13px;
}
.check {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
  font-size: 13px;
}
code {
  font-family: var(--font-mono);
  font-size: 12px;
}
</style>
