<script setup lang="ts">
import { useTunnelStore } from "../stores/tunnels";
import type { TunnelProfile } from "../types/tunnel";
import GroupSection from "./GroupSection.vue";

const store = useTunnelStore();

defineEmits<{
  edit: [profile: TunnelProfile];
  duplicate: [profile: TunnelProfile];
}>();
</script>

<template>
  <div v-if="store.profiles.length === 0" class="text-center py-12 px-5 text-muted text-sm bg-elevated border border-default rounded-lg">
    Todavía no hay perfiles. Crea uno para empezar a reenviar puertos.
  </div>

  <GroupSection
    v-for="group in store.groupedProfiles"
    :key="group.name ?? '__ungrouped'"
    :name="group.name"
    :profiles="group.profiles"
    @edit="$emit('edit', $event)"
    @duplicate="$emit('duplicate', $event)"
  />
</template>
