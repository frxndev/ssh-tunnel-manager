<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import type { TunnelLogLine } from "../types/tunnel";

const props = defineProps<{
  lines: TunnelLogLine[];
}>();

const emit = defineEmits<{ clear: [] }>();

const bodyRef = ref<HTMLElement | null>(null);

function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString(undefined, { hour12: false });
}

const levelClass: Record<TunnelLogLine["level"], string> = {
  info: "text-muted",
  warn: "text-warning",
  error: "text-error",
};

watch(
  () => props.lines.length,
  async () => {
    await nextTick();
    if (bodyRef.value) bodyRef.value.scrollTop = bodyRef.value.scrollHeight;
  },
);
</script>

<template>
  <div class="border-t border-default bg-elevated">
    <div class="flex justify-between items-center px-4 py-2 text-[11px] font-mono-data text-dimmed uppercase tracking-wide">
      <span>{{ lines.length }} líneas</span>
      <UButton size="xs" color="neutral" variant="outline" @click="emit('clear')">Limpiar</UButton>
    </div>
    <div ref="bodyRef" class="max-h-56 overflow-y-auto px-4 pb-3 font-mono-data text-xs">
      <p v-if="lines.length === 0" class="text-dimmed py-2">
        Sin actividad todavía. Inicia el túnel y probá conectarte para ver qué pasa acá.
      </p>
      <div v-for="(line, i) in lines" :key="i" class="flex gap-2.5 py-0.5 leading-relaxed">
        <span class="text-dimmed shrink-0">{{ formatTime(line.timestampMs) }}</span>
        <span class="wrap-break-word" :class="levelClass[line.level]">{{ line.message }}</span>
      </div>
    </div>
  </div>
</template>
