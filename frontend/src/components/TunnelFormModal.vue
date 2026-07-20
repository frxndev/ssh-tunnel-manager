<script setup lang="ts">
import { reactive, watch, computed } from "vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { useTunnelStore } from "../stores/tunnels";
import { createBlankProfile, type TunnelProfile } from "../types/tunnel";
import { usePortCheck } from "../composables/usePortCheck";
import { useConnectionTest } from "../composables/useConnectionTest";
import TemplatePicker from "./TemplatePicker.vue";

const isOpen = defineModel<boolean>("open", { required: true });

const props = defineProps<{
  profile: TunnelProfile | null; // the profile being edited/duplicated, or null for "new"
}>();
const emit = defineEmits<{ save: [profile: TunnelProfile] }>();

const store = useTunnelStore();

// A single reactive object that lives for the component's whole lifetime;
// switching between "new"/"edit"/"duplicate" mutates its fields in place
// (see the watch below) rather than replacing the object, which is what
// lets TemplatePicker hold a stable reference to it.
const form = reactive<TunnelProfile>(createBlankProfile());

watch(
  () => props.profile,
  (val) => Object.assign(form, createBlankProfile(), val ?? {}),
  { immediate: true },
);

const isEditing = computed(() => !!props.profile?.id);

const { portStatus, checkPort, resetPortStatus } = usePortCheck();
const { testing, result: testResult, testConnection } = useConnectionTest();

const kindItems = [
  { label: "Local forward (-L)", value: "Local" },
  { label: "Remote forward (-R)", value: "Remote" },
  { label: "Dinámico / SOCKS (-D)", value: "Dynamic" },
];
const authItems = [
  { label: "Clave privada", value: "PrivateKey" },
  { label: "Contraseña", value: "Password" },
  { label: "Agente SSH", value: "Agent" },
];

async function browseForKey() {
  const selected = await openFileDialog({ title: "Elegir clave privada SSH", multiple: false, directory: false });
  if (typeof selected === "string") form.privateKeyPath = selected;
}

function submit() {
  emit("save", {
    ...form,
    sshPort: Number(form.sshPort),
    localPort: Number(form.localPort),
    remotePort: form.remotePort != null ? Number(form.remotePort) : form.remotePort,
  });
}

function cancel() {
  isOpen.value = false;
}
</script>

<template>
  <UModal v-model:open="isOpen" :title="isEditing ? 'Editar túnel' : 'Nuevo túnel'" :ui="{ content: 'max-w-2xl' }">
    <template #body>
      <form class="flex flex-col gap-4" @submit.prevent="submit">
        <div class="flex gap-4">
          <UFormField label="Nombre del perfil" class="flex-1">
            <UInput v-model="form.name" required placeholder="p. ej. DB producción" class="w-full" />
          </UFormField>
          <UFormField label="Grupo (opcional)" class="flex-1">
            <UInput v-model="form.group" list="group-options" placeholder="p. ej. Producción" class="w-full" />
            <datalist id="group-options">
              <option v-for="g in store.groupNames" :key="g" :value="g" />
            </datalist>
          </UFormField>
        </div>

        <TemplatePicker :form="form" />

        <div class="flex gap-4">
          <UFormField label="Host SSH" class="flex-1">
            <UInput v-model="form.sshHost" required placeholder="bastion.miempresa.com" class="w-full" />
          </UFormField>
          <UFormField label="Puerto SSH" class="w-32">
            <UInput v-model.number="form.sshPort" type="number" :min="1" :max="65535" required class="w-full" />
          </UFormField>
        </div>

        <div class="flex gap-4">
          <UFormField label="Usuario" class="flex-1">
            <UInput v-model="form.sshUser" required placeholder="ubuntu" class="w-full" />
          </UFormField>
          <UFormField label="Autenticación" class="flex-1">
            <USelect v-model="form.authMethod" :items="authItems" value-key="value" class="w-full" />
          </UFormField>
        </div>

        <div v-if="form.authMethod === 'PrivateKey'" class="flex gap-4">
          <UFormField label="Ruta de la clave privada" class="flex-1">
            <div class="flex gap-2">
              <UInput v-model="form.privateKeyPath" placeholder="~/.ssh/id_ed25519" class="flex-1" />
              <UButton color="neutral" variant="outline" @click="browseForKey">Examinar…</UButton>
            </div>
          </UFormField>
          <UFormField label="Passphrase (opcional)" class="flex-1">
            <UInput v-model="form.passphrase" type="password" class="w-full" />
          </UFormField>
        </div>

        <UFormField v-if="form.authMethod === 'Password'" label="Contraseña">
          <UInput v-model="form.password" type="password" class="w-full" />
        </UFormField>

        <div class="flex flex-col items-start gap-2">
          <UButton
            color="primary"
            variant="link"
            size="sm"
            class="p-0"
            :loading="testing"
            :disabled="testing || !form.sshHost || !form.sshUser"
            @click="testConnection(form)"
          >
            ⚡ Probar conexión
          </UButton>
          <UAlert
            v-if="testResult"
            :color="testResult.ok ? 'success' : 'error'"
            variant="soft"
            :title="testResult.message"
            class="w-full"
          />
        </div>

        <UFormField label="Tipo de túnel">
          <USelect v-model="form.kind" :items="kindItems" value-key="value" class="w-full" @update:model-value="resetPortStatus" />
        </UFormField>

        <div class="flex gap-4">
          <UFormField :label="form.kind === 'Remote' ? 'Bind en el servidor remoto' : 'Host local'" class="flex-1">
            <UInput
              v-model="form.localHost"
              placeholder="127.0.0.1"
              class="w-full"
              @blur="checkPort(form.kind, form.localHost, form.localPort)"
            />
          </UFormField>
          <UFormField :label="`Puerto ${form.kind === 'Remote' ? 'remoto' : 'local'}`" class="w-32">
            <UInput
              v-model.number="form.localPort"
              type="number"
              :min="1"
              :max="65535"
              required
              class="w-full"
              @blur="checkPort(form.kind, form.localHost, form.localPort)"
            />
          </UFormField>
        </div>
        <p v-if="portStatus === 'checking'" class="text-xs text-dimmed -mt-2">Comprobando el puerto…</p>
        <p v-else-if="portStatus === 'busy'" class="text-xs text-warning -mt-2">
          ⚠ Ese puerto ya está en uso en tu equipo — el túnel probablemente no va a poder iniciar hasta que liberes el puerto o elijas otro.
        </p>
        <p v-else-if="portStatus === 'free'" class="text-xs text-success -mt-2">✓ Puerto disponible.</p>

        <div v-if="form.kind !== 'Dynamic'" class="flex gap-4">
          <UFormField label="Host destino" class="flex-1">
            <UInput v-model="form.remoteHost" required placeholder="db.internal" class="w-full" />
          </UFormField>
          <UFormField label="Puerto destino" class="w-32">
            <UInput v-model.number="form.remotePort" type="number" :min="1" :max="65535" required class="w-full" />
          </UFormField>
        </div>
      </form>
    </template>

    <template #footer>
      <div class="flex justify-end gap-2 w-full">
        <UButton color="neutral" variant="outline" @click="cancel">Cancelar</UButton>
        <UButton color="primary" @click="submit">Guardar perfil</UButton>
      </div>
    </template>
  </UModal>
</template>
