<script setup lang="ts">
import { ref } from "vue";
import type { TunnelProfile } from "../types/tunnel";
import TunnelCard from "./TunnelCard.vue";

defineProps<{
  name: string | null;
  profiles: TunnelProfile[];
}>();

defineEmits<{
  edit: [profile: TunnelProfile];
  duplicate: [profile: TunnelProfile];
}>();

const collapsed = ref(false);
</script>

<template>
  <div class="mb-1">
    <button
      v-if="name"
      class="flex items-center gap-2 w-full text-left py-2.5 px-1 text-xs font-bold uppercase tracking-wide text-muted hover:text-highlighted font-mono-data"
      @click="collapsed = !collapsed"
    >
      <UIcon name="i-lucide-chevron-down" class="transition-transform" :class="{ '-rotate-90': collapsed }" />
      {{ name }}
      <UBadge color="neutral" variant="subtle" size="sm" class="ml-auto">{{ profiles.length }}</UBadge>
    </button>

    <div v-if="!name || !collapsed" class="flex flex-col gap-2 mb-2">
      <TunnelCard v-for="p in profiles" :key="p.id" :profile="p" @edit="$emit('edit', $event)" @duplicate="$emit('duplicate', $event)" />
    </div>
  </div>
</template>
