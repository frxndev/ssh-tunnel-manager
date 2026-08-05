<script setup lang="ts">
import { computed } from "vue";
import type { TunnelKind } from "../types/tunnel";

const props = defineProps<{ kind: TunnelKind }>();

const badge: Record<TunnelKind, string> = { Local: "L", Remote: "R", Dynamic: "D" };
const label: Record<TunnelKind, string> = {
  Local: "Local forward",
  Remote: "Remote forward",
  Dynamic: "SOCKS dinámico",
};
const color: Record<TunnelKind, "primary" | "secondary" | "warning"> = {
  Local: "primary",
  Remote: "secondary",
  Dynamic: "warning",
};

const text = computed(() => badge[props.kind]);
const tooltip = computed(() => label[props.kind]);
const badgeColor = computed(() => color[props.kind]);
</script>

<template>
  <UTooltip :text="tooltip">
    <UBadge :color="badgeColor" variant="subtle" size="lg" class="font-mono-data font-bold w-8 justify-center">
      {{ text }}
    </UBadge>
  </UTooltip>
</template>
