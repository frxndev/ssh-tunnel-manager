<script setup lang="ts">
import { ref } from "vue";
import { useTunnelStore } from "../stores/tunnels";
import type { TunnelProfile } from "../types/tunnel";
import TunnelKindBadge from "./TunnelKindBadge.vue";
import TunnelStatusIndicator from "./TunnelStatusIndicator.vue";
import LogConsole from "./LogConsole.vue";

const props = defineProps<{ profile: TunnelProfile }>();
const emit = defineEmits<{
  edit: [profile: TunnelProfile];
  duplicate: [profile: TunnelProfile];
}>();

const store = useTunnelStore();
const showLogs = ref(false);

async function toggleRunning() {
  if (store.isRunning(props.profile.id)) {
    await store.stopTunnel(props.profile.id);
  } else {
    try {
      await store.startTunnel(props.profile.id);
    } catch {
      // error surfaced via store.errors[id], shown inline below
    }
  }
}

function formatBytes(bytes: number): string {
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
</script>

<template>
  <div class="bg-elevated border border-default rounded-lg overflow-hidden transition-colors hover:border-(--ui-text-dimmed)">
    <div class="flex items-center gap-3.5 p-4">
      <TunnelKindBadge :kind="profile.kind" />

      <div class="flex-1 min-w-0">
        <div class="flex items-baseline gap-2">
          <strong class="text-highlighted">{{ profile.name }}</strong>
        </div>
        <div class="text-xs text-muted mt-0.5 font-mono-data wrap-break-word">
          {{ profile.sshUser }}@{{ profile.sshHost }}:{{ profile.sshPort }}
          <template v-if="profile.kind !== 'Dynamic'">
            → {{ profile.localHost }}:{{ profile.localPort }} ⇢ {{ profile.remoteHost }}:{{ profile.remotePort }}
          </template>
          <template v-else>→ SOCKS en {{ profile.localHost }}:{{ profile.localPort }}</template>
        </div>
        <div v-if="store.isRunning(profile.id)" class="text-[11px] text-primary mt-0.5 font-mono-data">
          {{ store.statsFor(profile.id).activeConnections }} conexión(es) activa(s) · ↓
          {{ formatBytes(store.statsFor(profile.id).bytesIn) }} · ↑ {{ formatBytes(store.statsFor(profile.id).bytesOut) }}
        </div>
        <p v-if="store.errors[profile.id]" class="text-xs text-error mt-1">{{ store.errors[profile.id] }}</p>
      </div>

      <div class="flex items-center gap-2 shrink-0">
        <TunnelStatusIndicator :status="store.status[profile.id] ?? 'stopped'" />
        <UButton
          size="sm"
          :color="store.isRunning(profile.id) ? 'error' : 'neutral'"
          :variant="store.isRunning(profile.id) ? 'solid' : 'outline'"
          @click="toggleRunning"
        >
          {{ store.isRunning(profile.id) ? "Detener" : "Iniciar" }}
        </UButton>
        <UTooltip text="Ver logs">
          <UButton
            size="sm"
            :color="showLogs ? 'primary' : 'neutral'"
            variant="ghost"
            icon="i-lucide-terminal"
            @click="showLogs = !showLogs"
          />
        </UTooltip>
        <UTooltip text="Duplicar">
          <UButton size="sm" color="neutral" variant="ghost" icon="i-lucide-copy" @click="emit('duplicate', profile)" />
        </UTooltip>
        <UTooltip text="Editar">
          <UButton size="sm" color="neutral" variant="ghost" icon="i-lucide-pencil" @click="emit('edit', profile)" />
        </UTooltip>
        <UTooltip text="Eliminar">
          <UButton
            size="sm"
            color="neutral"
            variant="ghost"
            icon="i-lucide-trash-2"
            class="hover:text-error"
            @click="store.deleteProfile(profile.id)"
          />
        </UTooltip>
      </div>
    </div>

    <LogConsole v-if="showLogs" :lines="store.logsFor(profile.id)" @clear="store.clearLogs(profile.id)" />
  </div>
</template>
