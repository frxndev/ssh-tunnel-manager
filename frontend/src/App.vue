<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { save as saveDialog, open as openDialog } from "@tauri-apps/plugin-dialog";
import { useTunnelStore } from "./stores/tunnels";
import type { TunnelProfile } from "./types/tunnel";
import TunnelFormModal from "./components/TunnelFormModal.vue";
import TunnelList from "./components/TunnelList.vue";

const store = useTunnelStore();
const showForm = ref(false);
const editing = ref<TunnelProfile | null>(null);
const importExportMessage = ref("");

const activeCount = computed(() => Object.values(store.status).filter((s) => s === "connected").length);

onMounted(() => store.init());

function openNew() {
  editing.value = null;
  showForm.value = true;
}
function openEdit(profile: TunnelProfile) {
  editing.value = profile;
  showForm.value = true;
}
function openDuplicate(profile: TunnelProfile) {
  // Same data, but no id (so saveProfile treats it as brand-new) and a name
  // that signals it's a copy — the person can rename it before saving.
  editing.value = { ...profile, id: "", name: `${profile.name} (copia)` };
  showForm.value = true;
}
async function handleSave(profile: TunnelProfile) {
  await store.saveProfile(profile);
  showForm.value = false;
}

function flashMessage(msg: string) {
  importExportMessage.value = msg;
  setTimeout(() => (importExportMessage.value = ""), 6000);
}

async function handleExport() {
  const path = await saveDialog({
    title: "Exportar perfiles",
    defaultPath: "perfiles-ssh.json",
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  await store.exportProfilesToFile(path);
  flashMessage(`Perfiles exportados a ${path} (sin contraseñas ni passphrases).`);
}

async function handleImport() {
  const path = await openDialog({
    title: "Importar perfiles",
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  const count = await store.importProfilesFromFile(path as string);
  flashMessage(`${count} perfil(es) importado(s). Recordá completar contraseñas si corresponde.`);
}
</script>

<template>
  <UApp>
    <div class="flex h-screen overflow-hidden bg-default text-default">
      <aside class="w-14 shrink-0 bg-elevated border-r border-default flex flex-col items-center py-3.5 gap-2.5">
        <div class="w-7.5 h-7.5 rounded-lg bg-primary text-inverted font-mono-data font-bold flex items-center justify-center mb-2">T</div>
        <UTooltip text="Túneles">
          <UButton color="primary" variant="soft" icon="i-lucide-cable" square />
        </UTooltip>
        <UTooltip text="Configuración (próximamente)">
          <UButton color="neutral" variant="ghost" icon="i-lucide-settings" square disabled />
        </UTooltip>
      </aside>

      <div class="flex-1 flex flex-col min-w-0 overflow-hidden">
        <header class="border-b border-default shrink-0">
          <div class="flex items-center justify-center gap-1 py-5 font-mono-data text-xs text-muted tracking-wide">
            <span class="px-3 py-1.5 border border-default rounded bg-elevated">local</span>
            <span class="w-16 h-px bg-default" />
            <span class="px-3 py-1.5 border border-primary text-primary rounded">ssh</span>
            <span class="w-16 h-px bg-default" />
            <span class="px-3 py-1.5 border border-default rounded bg-elevated">remoto</span>
          </div>

          <div class="flex items-end justify-between px-7 pb-4.5 max-w-3xl mx-auto">
            <div>
              <h1 class="font-mono-data text-xl font-semibold">SSH Tunnel Manager</h1>
              <p class="text-muted text-sm mt-1">{{ activeCount }} de {{ store.profiles.length }} perfiles activos</p>
            </div>
            <div class="flex items-center gap-2">
              <UButton color="neutral" variant="outline" @click="handleImport">Importar</UButton>
              <UButton color="neutral" variant="outline" @click="handleExport">Exportar</UButton>
              <UButton color="primary" @click="openNew">+ Nuevo túnel</UButton>
            </div>
          </div>
          <UAlert
            v-if="importExportMessage"
            color="success"
            variant="subtle"
            :title="importExportMessage"
            class="max-w-3xl mx-auto mb-3.5"
          />
        </header>

        <main class="flex-1 overflow-y-auto px-7 py-6 max-w-3xl w-full mx-auto">
          <TunnelList @edit="openEdit" @duplicate="openDuplicate" />
        </main>
      </div>

      <TunnelFormModal v-model:open="showForm" :profile="editing" @save="handleSave" />
    </div>
  </UApp>
</template>
