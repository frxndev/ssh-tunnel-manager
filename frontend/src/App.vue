<script setup>
import { ref, computed, onMounted } from "vue";
import { save, open } from "@tauri-apps/plugin-dialog";
import { useTunnelStore } from "./stores/tunnels";
import TunnelForm from "./components/TunnelForm.vue";
import TunnelList from "./components/TunnelList.vue";

const store = useTunnelStore();
const showForm = ref(false);
const editing = ref(null);
const importExportMessage = ref("");

const activeCount = computed(() => Object.values(store.status).filter((s) => s === "connected").length);

onMounted(() => store.init());

function openNew() {
  editing.value = null;
  showForm.value = true;
}
function openEdit(profile) {
  editing.value = profile;
  showForm.value = true;
}
function openDuplicate(profile) {
  // Same data, but no id (so saveProfile treats it as brand-new) and a
  // name that signals it's a copy — the person can rename it before saving.
  editing.value = { ...profile, id: "", name: `${profile.name} (copia)` };
  showForm.value = true;
}
async function handleSave(profile) {
  await store.saveProfile(profile);
  showForm.value = false;
}

async function handleExport() {
  const path = await save({
    title: "Exportar perfiles",
    defaultPath: "perfiles-ssh.json",
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  await store.exportProfilesToFile(path);
  importExportMessage.value = `Perfiles exportados a ${path} (sin contraseñas ni passphrases).`;
  setTimeout(() => (importExportMessage.value = ""), 6000);
}

async function handleImport() {
  const path = await open({
    title: "Importar perfiles",
    multiple: false,
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return;
  const count = await store.importProfilesFromFile(path);
  importExportMessage.value = `${count} perfil(es) importado(s). Recordá completar contraseñas si corresponde.`;
  setTimeout(() => (importExportMessage.value = ""), 6000);
}
</script>

<template>
  <aside class="rail">
    <div class="rail-logo">T</div>
    <button class="rail-btn active" title="Túneles">
      <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
        <circle cx="5" cy="12" r="2.5" />
        <circle cx="19" cy="12" r="2.5" />
        <path d="M7.5 12h9" stroke-dasharray="2 2" />
      </svg>
    </button>
    <button class="rail-btn" title="Configuración" disabled>
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
        <circle cx="12" cy="12" r="3" />
        <path
          d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 11-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 11-4 0v-.09a1.65 1.65 0 00-1-1.51 1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 11-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 110-4h.09a1.65 1.65 0 001.51-1 1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 112.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 114 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 112.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 110 4h-.09a1.65 1.65 0 00-1.51 1z"
        />
      </svg>
    </button>
  </aside>

  <div class="main">
    <header class="header">
      <div class="signal" :class="{ live: activeCount > 0 }">
        <span class="node">local</span>
        <span class="wire"><span class="pulse"></span></span>
        <span class="node mid">ssh</span>
        <span class="wire"><span class="pulse" style="animation-delay: 0.4s"></span></span>
        <span class="node">remoto</span>
      </div>
      <div class="title-row">
        <div>
          <h1>Túneles SSH</h1>
          <p class="subtitle">{{ activeCount }} de {{ store.profiles.length }} perfiles activos</p>
        </div>
        <div class="header-actions">
          <button class="ghost-btn" title="Importar perfiles desde un archivo" @click="handleImport">Importar</button>
          <button class="ghost-btn" title="Exportar perfiles a un archivo" @click="handleExport">Exportar</button>
          <button class="new-btn" @click="openNew">+ Nuevo túnel</button>
        </div>
      </div>
      <div v-if="importExportMessage" class="toast">{{ importExportMessage }}</div>
    </header>

    <main class="content">
      <TunnelForm v-if="showForm" :model-value="editing" @save="handleSave" @cancel="showForm = false" />
      <TunnelList v-else @edit="openEdit" @duplicate="openDuplicate" />
    </main>
  </div>
</template>

<style scoped>
.rail {
  width: 56px;
  flex-shrink: 0;
  background: var(--surface);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 14px 0;
  gap: 10px;
}
.rail-logo {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  background: var(--accent);
  color: #06201c;
  font-family: var(--font-display);
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 8px;
}
.rail-btn {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}
.rail-btn:hover:not(:disabled) {
  background: var(--surface-hover);
  color: var(--text);
}
.rail-btn.active {
  background: var(--surface-hover);
  color: var(--accent);
}
.rail-btn:disabled {
  opacity: 0.35;
  cursor: default;
}

.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
}

.header {
  border-bottom: 1px solid var(--border);
  background: var(--bg);
  flex-shrink: 0;
}

.content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 28px 40px;
  max-width: 860px;
  width: 100%;
  margin: 0 auto;
}

.signal {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 20px 0 22px;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-muted);
  letter-spacing: 0.04em;
}
.node {
  padding: 5px 12px;
  border: 1px solid var(--border);
  border-radius: 5px;
  background: var(--surface);
}
.node.mid {
  color: var(--accent);
  border-color: var(--accent);
}
.wire {
  position: relative;
  width: 64px;
  height: 1px;
  background: var(--border);
}
.pulse {
  position: absolute;
  top: -2px;
  left: 0;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--text-muted);
  opacity: 0;
}
.signal.live .pulse {
  background: var(--accent);
  animation: travel 1.6s infinite ease-in-out;
}
@keyframes travel {
  0% {
    left: 0;
    opacity: 0;
  }
  15% {
    opacity: 1;
  }
  85% {
    opacity: 1;
  }
  100% {
    left: 58px;
    opacity: 0;
  }
}

.title-row {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  padding: 0 28px 18px;
  max-width: 780px;
  margin: 0 auto;
}
h1 {
  font-family: var(--font-display);
  font-size: 21px;
  margin: 0;
  font-weight: 600;
}
.subtitle {
  margin: 4px 0 0;
  color: var(--text-muted);
  font-size: 13px;
}
.new-btn {
  padding: 10px 18px;
  background: var(--accent);
  color: #06201c;
  border: none;
  border-radius: 7px;
  font-weight: 700;
  font-size: 13px;
  cursor: pointer;
}
.new-btn:hover {
  background: var(--accent-hover);
}
.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ghost-btn {
  padding: 10px 14px;
  background: transparent;
  color: var(--text-muted);
  border: 1px solid var(--border);
  border-radius: 7px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}
.ghost-btn:hover {
  color: var(--text);
  border-color: var(--text-faint);
}
.toast {
  max-width: 780px;
  margin: 0 auto 14px;
  padding: 10px 16px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--success);
  font-size: 13px;
}
</style>
