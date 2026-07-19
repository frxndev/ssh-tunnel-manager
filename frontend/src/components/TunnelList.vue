<script setup>
import { ref, nextTick, watch } from "vue";
import { useTunnelStore } from "../stores/tunnels";

const store = useTunnelStore();
defineEmits(["edit", "duplicate"]);

const kindBadge = { Local: "L", Remote: "R", Dynamic: "D" };
const kindLabel = { Local: "Local forward", Remote: "Remote forward", Dynamic: "SOCKS dinámico" };

const expanded = ref(new Set());
const logRefs = ref({});
const collapsedGroups = ref(new Set());

function statusLabel(id) {
  return { stopped: "Detenido", connecting: "Conectando…", connected: "Activo", error: "Error" }[store.status[id] ?? "stopped"];
}

async function toggle(profile) {
  if (store.isRunning(profile.id)) {
    await store.stopTunnel(profile.id);
  } else {
    try {
      await store.startTunnel(profile.id);
    } catch {
      // error surfaced via store.errors[id], shown inline below
    }
  }
}

function toggleLogs(id) {
  const next = new Set(expanded.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  expanded.value = next;
}

function toggleGroup(name) {
  const next = new Set(collapsedGroups.value);
  if (next.has(name)) next.delete(name);
  else next.add(name);
  collapsedGroups.value = next;
}

function formatTime(ms) {
  return new Date(ms).toLocaleTimeString(undefined, { hour12: false });
}

function formatBytes(bytes) {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let n = bytes;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

// Auto-scroll each open console to the bottom as new lines arrive.
watch(
  () => JSON.stringify(store.logs),
  async () => {
    await nextTick();
    for (const id of expanded.value) {
      const el = logRefs.value[id];
      if (el) el.scrollTop = el.scrollHeight;
    }
  },
);
</script>

<template>
  <div v-if="store.profiles.length === 0" class="empty">Todavía no hay perfiles. Crea uno para empezar a reenviar puertos.</div>

  <div v-for="group in store.groupedProfiles" :key="group.name ?? '__ungrouped'" class="group-section">
    <button v-if="group.name" class="group-header" @click="toggleGroup(group.name)">
      <span class="chevron" :class="{ collapsed: collapsedGroups.has(group.name) }">▾</span>
      {{ group.name }}
      <span class="group-count">{{ group.profiles.length }}</span>
    </button>

    <ul v-if="!group.name || !collapsedGroups.has(group.name)" class="tunnel-list">
      <li v-for="p in group.profiles" :key="p.id" class="tunnel-item">
        <div class="tunnel-row">
          <div class="badge" :class="p.kind.toLowerCase()">{{ kindBadge[p.kind] }}</div>

          <div class="info">
            <div class="name-line">
              <strong>{{ p.name }}</strong>
              <span class="kind">{{ kindLabel[p.kind] }}</span>
            </div>
            <div class="detail">
              <span style="display: block">{{ p.sshUser }}@{{ p.sshHost }}:{{ p.sshPort }}</span>
              <span v-if="p.kind !== 'Dynamic'" style="display: block">
                {{ p.localHost }}:{{ p.localPort }} ⇢ {{ p.remoteHost }}:{{ p.remotePort }}
              </span>
              <span v-else style="display: block">SOCKS en {{ p.localHost }}:{{ p.localPort }}</span>
            </div>
            <div v-if="store.isRunning(p.id)" class="stats-line">
              {{ store.statsFor(p.id).activeConnections }} conexión(es) activa(s)
              <span style="display: block">
                ↓ {{ formatBytes(store.statsFor(p.id).bytesIn) }} · ↑ {{ formatBytes(store.statsFor(p.id).bytesOut) }}
              </span>
            </div>
            <div v-if="store.errors[p.id]" class="error-msg">{{ store.errors[p.id] }}</div>
          </div>

          <div class="row-actions">
            <span class="status" :class="store.status[p.id]"> <span class="status-dot"></span>{{ statusLabel(p.id) }} </span>
            <button class="toggle" :class="{ on: store.isRunning(p.id) }" @click="toggle(p)">
              {{ store.isRunning(p.id) ? "Detener" : "Iniciar" }}
            </button>
            <button class="icon-btn" :class="{ active: expanded.has(p.id) }" title="Ver logs" @click="toggleLogs(p.id)">⌘ Logs</button>
            <button class="icon-btn" title="Duplicar" @click="$emit('duplicate', p)">⧉</button>
            <button class="icon-btn" title="Editar" @click="$emit('edit', p)">✎</button>
            <button class="icon-btn danger" title="Eliminar" @click="store.deleteProfile(p.id)">🗑</button>
          </div>
        </div>

        <div v-if="expanded.has(p.id)" class="console">
          <div class="console-header">
            <span>{{ store.logsFor(p.id).length }} líneas</span>
            <button class="clear-btn" @click="store.clearLogs(p.id)">Limpiar</button>
          </div>
          <div class="console-body" :ref="(el) => (logRefs[p.id] = el)">
            <div v-if="store.logsFor(p.id).length === 0" class="console-empty">
              Sin actividad todavía. Inicia el túnel y probá conectarte para ver qué pasa acá.
            </div>
            <div v-for="(line, i) in store.logsFor(p.id)" :key="i" class="console-line" :class="line.level">
              <span class="ts">{{ formatTime(line.timestampMs) }}</span>
              <span class="msg">{{ line.message }}</span>
            </div>
          </div>
        </div>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.group-section {
  margin-bottom: 4px;
}
.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  background: none;
  border: none;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 10px 4px;
  cursor: pointer;
  width: 100%;
  text-align: left;
}
.group-header:hover {
  color: var(--text);
}
.chevron {
  display: inline-block;
  transition: transform 0.15s;
  font-size: 10px;
}
.chevron.collapsed {
  transform: rotate(-90deg);
}
.group-count {
  margin-left: auto;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 1px 8px;
  font-size: 11px;
}

.tunnel-list {
  list-style: none;
  margin: 0 0 8px;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.tunnel-item {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  transition: border-color 0.15s;
  overflow: hidden;
}
.tunnel-item:hover {
  border-color: var(--text-faint);
}

.tunnel-row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
}

.badge {
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: var(--font-mono);
  font-weight: 700;
  font-size: 14px;
}
.badge.local {
  background: rgba(45, 212, 191, 0.15);
  color: var(--accent);
}
.badge.remote {
  background: rgba(168, 130, 250, 0.15);
  color: #a882fa;
}
.badge.dynamic {
  background: rgba(251, 191, 36, 0.15);
  color: var(--warning);
}

.info {
  flex: 1;
  min-width: 0;
}
.name-line {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.kind {
  font-size: 11px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}
.detail {
  font-size: 13px;
  color: var(--text-muted);
  margin-top: 3px;
  font-family: var(--font-mono);
  overflow-wrap: anywhere;
}
.stats-line {
  font-size: 11px;
  color: var(--accent);
  margin-top: 3px;
  font-family: var(--font-mono);
}
.error-msg {
  font-size: 12px;
  color: var(--danger);
  margin-top: 4px;
}

.row-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.status {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-muted);
  width: 92px;
}
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-faint);
  flex-shrink: 0;
}
.status.connected .status-dot {
  background: var(--success);
  box-shadow: 0 0 0 3px rgba(52, 211, 153, 0.2);
}
.status.connected {
  color: var(--success);
}
.status.connecting .status-dot {
  background: var(--warning);
  animation: pulse 1.2s infinite;
}
.status.connecting {
  color: var(--warning);
}
.status.error .status-dot {
  background: var(--danger);
}
.status.error {
  color: var(--danger);
}
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
  }
}

button {
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-weight: 600;
}
.toggle {
  padding: 8px 14px;
  background: var(--surface-inset);
  color: var(--text);
  border: 1px solid var(--border);
  font-size: 13px;
  white-space: nowrap;
}
.toggle:hover {
  border-color: var(--text-faint);
}
.toggle.on {
  background: var(--danger);
  color: white;
  border-color: transparent;
}
.icon-btn {
  background: transparent;
  color: var(--text-muted);
  padding: 7px 10px;
  font-size: 12px;
  white-space: nowrap;
}
.icon-btn:hover {
  color: var(--text);
  background: var(--surface-inset);
}
.icon-btn.active {
  color: var(--accent);
  background: var(--surface-inset);
}
.icon-btn.danger:hover {
  color: var(--danger);
}

.console {
  border-top: 1px solid var(--border);
  background: var(--surface-inset);
}
.console-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 16px;
  font-size: 11px;
  color: var(--text-faint);
  font-family: var(--font-mono);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.clear-btn {
  background: transparent;
  color: var(--text-muted);
  font-size: 11px;
  padding: 3px 8px;
  border: 1px solid var(--border);
}
.clear-btn:hover {
  color: var(--text);
}
.console-body {
  max-height: 220px;
  overflow-y: auto;
  padding: 0 16px 12px;
  font-family: var(--font-mono);
  font-size: 12px;
}
.console-empty {
  color: var(--text-faint);
  padding: 8px 0;
}
.console-line {
  display: flex;
  gap: 10px;
  padding: 2px 0;
  line-height: 1.5;
}
.console-line .ts {
  color: var(--text-faint);
  flex-shrink: 0;
}
.console-line .msg {
  color: var(--text-muted);
  overflow-wrap: anywhere;
}
.console-line.warn .msg {
  color: var(--warning);
}
.console-line.error .msg {
  color: var(--danger);
}

.empty {
  text-align: center;
  padding: 48px 20px;
  color: var(--text-muted);
  font-size: 14px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
}
</style>
