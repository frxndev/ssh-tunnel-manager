<script setup lang="ts">
import { useTunnelStore } from "../stores/tunnels";
import { useSshTemplates } from "../composables/useSshTemplates";
import type { TunnelProfile } from "../types/tunnel";

// `form` is the parent's reactive() form object, passed by reference. This
// component (via useSshTemplates) mutates its fields directly instead of
// using v-model — the parent's `form` is never reassigned wholesale, only
// its properties change in place, so a plain prop is the honest contract
// here rather than v-model's "the child may replace the whole value" one.
const props = defineProps<{ form: TunnelProfile }>();

const store = useTunnelStore();
const { selectedTemplateId, showSaveTemplate, newTemplateName, applyTemplate, saveAsTemplate, deleteSelectedTemplate } = useSshTemplates(
  props.form,
);
</script>

<template>
  <div v-if="store.templates.length > 0" class="p-3 bg-accented rounded-lg border border-default mb-1">
    <label class="block text-[11px] font-bold uppercase tracking-wide text-muted font-mono-data mb-1.5">
      Usar plantilla de conexión SSH
    </label>
    <div class="flex gap-2">
      <USelect
        v-model="selectedTemplateId"
        :items="store.templates.map((t) => ({ label: t.name, value: t.id }))"
        placeholder="— elegir plantilla —"
        class="flex-1"
        @update:model-value="applyTemplate"
      />
      <UButton v-if="selectedTemplateId" color="error" variant="soft" @click="deleteSelectedTemplate"> Eliminar </UButton>
    </div>
  </div>

  <div class="flex flex-col items-start gap-2 mb-1">
    <UButton color="primary" variant="link" size="sm" class="p-0" @click="showSaveTemplate = !showSaveTemplate">
      + Guardar estos datos de conexión como plantilla
    </UButton>
    <div v-if="showSaveTemplate" class="flex gap-2 w-full">
      <UInput v-model="newTemplateName" placeholder="Nombre de la plantilla (p. ej. Bastión producción)" class="flex-1" />
      <UButton color="neutral" variant="outline" @click="saveAsTemplate">Guardar plantilla</UButton>
    </div>
  </div>
</template>
