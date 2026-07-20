<script setup lang="ts">
import { computed } from "vue";
import type { TunnelRunStatus } from "../types/tunnel";

const props = defineProps<{ status: TunnelRunStatus }>();

const label: Record<TunnelRunStatus, string> = {
  stopped: "Detenido",
  connecting: "Conectando…",
  connected: "Activo",
  error: "Error",
};

const dotClass: Record<TunnelRunStatus, string> = {
  stopped: "bg-(--ui-text-dimmed)",
  connecting: "bg-warning animate-pulse",
  connected: "bg-success shadow-[0_0_0_3px_rgba(52,211,153,0.2)]",
  error: "bg-error",
};

const textClass: Record<TunnelRunStatus, string> = {
  stopped: "text-muted",
  connecting: "text-warning",
  connected: "text-success",
  error: "text-error",
};

const text = computed(() => label[props.status]);
</script>

<template>
  <span class="inline-flex items-center gap-1.5 text-xs w-24" :class="textClass[status]">
    <span class="w-2 h-2 rounded-full shrink-0" :class="dotClass[status]" />
    {{ text }}
  </span>
</template>
