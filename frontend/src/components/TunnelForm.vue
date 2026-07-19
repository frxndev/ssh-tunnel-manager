<script setup>
import { reactive, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useTunnelStore } from '../stores/tunnels'

const store = useTunnelStore()

const props = defineProps({
  modelValue: { type: Object, default: null }, // profile being edited, or null for "new"
})
const emit = defineEmits(['save', 'cancel'])

function blankProfile() {
  return {
    id: '',
    name: '',
    kind: 'Local',
    group: '',
    sshHost: '',
    sshPort: 22,
    sshUser: '',
    authMethod: 'PrivateKey',
    password: '',
    privateKeyPath: '',
    passphrase: '',
    localHost: '127.0.0.1',
    localPort: 8080,
    remoteHost: '',
    remotePort: 80,
  }
}

const form = reactive(blankProfile())

watch(
  () => props.modelValue,
  (val) => Object.assign(form, blankProfile(), val ?? {}),
  { immediate: true }
)

function submit() {
  emit('save', { ...form })
}

async function browseForKey() {
  const selected = await open({
    title: 'Elegir clave privada SSH',
    multiple: false,
    directory: false,
  })
  if (typeof selected === 'string') {
    form.privateKeyPath = selected
  }
}

// -- SSH connection templates ----------------------------------------

const selectedTemplateId = ref('')
const showSaveTemplate = ref(false)
const newTemplateName = ref('')

function applyTemplate() {
  const tpl = store.templates.find((t) => t.id === selectedTemplateId.value)
  if (!tpl) return
  form.sshHost = tpl.sshHost
  form.sshPort = tpl.sshPort
  form.sshUser = tpl.sshUser
  form.authMethod = tpl.authMethod
  form.privateKeyPath = tpl.privateKeyPath || ''
}

async function saveAsTemplate() {
  if (!newTemplateName.value.trim()) return
  await store.saveTemplate({
    id: '',
    name: newTemplateName.value.trim(),
    sshHost: form.sshHost,
    sshPort: form.sshPort,
    sshUser: form.sshUser,
    authMethod: form.authMethod,
    privateKeyPath: form.authMethod === 'PrivateKey' ? form.privateKeyPath : null,
  })
  newTemplateName.value = ''
  showSaveTemplate.value = false
}

// -- Port availability check ------------------------------------------

const portStatus = ref(null) // null | 'checking' | 'free' | 'busy'

async function checkPort() {
  if (form.kind === 'Remote') {
    portStatus.value = null // that port is bound on the *server*, not here
    return
  }
  portStatus.value = 'checking'
  try {
    const available = await store.checkPortAvailable(form.localHost || '127.0.0.1', form.localPort)
    portStatus.value = available ? 'free' : 'busy'
  } catch {
    portStatus.value = null
  }
}

// -- Test connection ----------------------------------------------------

const testing = ref(false)
const testResult = ref(null) // { ok: boolean, message: string } | null

async function testConnection() {
  testing.value = true
  testResult.value = null
  try {
    const message = await store.testConnection({ ...form })
    testResult.value = { ok: true, message }
  } catch (err) {
    testResult.value = { ok: false, message: String(err) }
  } finally {
    testing.value = false
  }
}
</script>

<template>
  <form class="tunnel-form" @submit.prevent="submit">
    <div class="field-row">
      <div class="field">
        <label>Nombre del perfil</label>
        <input v-model="form.name" required placeholder="p. ej. DB producción" />
      </div>
      <div class="field">
        <label>Grupo (opcional)</label>
        <input v-model="form.group" list="group-options" placeholder="p. ej. Producción" />
        <datalist id="group-options">
          <option v-for="g in store.groupNames" :key="g" :value="g" />
        </datalist>
      </div>
    </div>

    <div v-if="store.templates.length > 0" class="field template-picker">
      <label>Usar plantilla de conexión SSH</label>
      <div class="input-with-button">
        <select v-model="selectedTemplateId" @change="applyTemplate">
          <option value="">— elegir plantilla —</option>
          <option v-for="t in store.templates" :key="t.id" :value="t.id">{{ t.name }}</option>
        </select>
        <button
          v-if="selectedTemplateId"
          type="button"
          class="browse-btn danger"
          title="Eliminar esta plantilla"
          @click="store.deleteTemplate(selectedTemplateId); selectedTemplateId = ''"
        >
          Eliminar
        </button>
      </div>
    </div>

    <div class="field-row">
      <div class="field">
        <label>Host SSH</label>
        <input v-model="form.sshHost" required placeholder="bastion.miempresa.com" />
      </div>
      <div class="field field-small">
        <label>Puerto SSH</label>
        <input v-model.number="form.sshPort" type="number" min="1" max="65535" required />
      </div>
    </div>

    <div class="field-row">
      <div class="field">
        <label>Usuario</label>
        <input v-model="form.sshUser" required placeholder="ubuntu" />
      </div>
      <div class="field">
        <label>Autenticación</label>
        <select v-model="form.authMethod">
          <option value="PrivateKey">Clave privada</option>
          <option value="Password">Contraseña</option>
          <option value="Agent">Agente SSH</option>
        </select>
      </div>
    </div>

    <div v-if="form.authMethod === 'PrivateKey'" class="field-row">
      <div class="field">
        <label>Ruta de la clave privada</label>
        <div class="input-with-button">
          <input v-model="form.privateKeyPath" placeholder="~/.ssh/id_ed25519" />
          <button type="button" class="browse-btn" @click="browseForKey">Examinar…</button>
        </div>
      </div>
      <div class="field">
        <label>Passphrase (opcional)</label>
        <input v-model="form.passphrase" type="password" />
      </div>
    </div>

    <div v-if="form.authMethod === 'Password'" class="field">
      <label>Contraseña</label>
      <input v-model="form.password" type="password" />
    </div>

    <div class="template-actions">
      <button type="button" class="link-btn" @click="showSaveTemplate = !showSaveTemplate">
        + Guardar estos datos de conexión como plantilla
      </button>
      <div v-if="showSaveTemplate" class="input-with-button">
        <input v-model="newTemplateName" placeholder="Nombre de la plantilla (p. ej. Bastión producción)" />
        <button type="button" class="browse-btn" @click="saveAsTemplate">Guardar plantilla</button>
      </div>

      <button type="button" class="link-btn" :disabled="testing || !form.sshHost || !form.sshUser" @click="testConnection">
        {{ testing ? 'Probando conexión…' : '⚡ Probar conexión' }}
      </button>
      <div v-if="testResult" class="test-result" :class="{ ok: testResult.ok, fail: !testResult.ok }">
        {{ testResult.message }}
      </div>
    </div>

    <div class="field">
      <label>Tipo de túnel</label>
      <select v-model="form.kind" @change="portStatus = null">
        <option value="Local">Local forward (-L)</option>
        <option value="Remote">Remote forward (-R)</option>
        <option value="Dynamic">Dinámico / SOCKS (-D)</option>
      </select>
    </div>

    <div class="field-row">
      <div class="field">
        <label>{{ form.kind === 'Remote' ? 'Bind en el servidor remoto' : 'Host local' }}</label>
        <input v-model="form.localHost" placeholder="127.0.0.1" @blur="checkPort" />
      </div>
      <div class="field field-small">
        <label>Puerto {{ form.kind === 'Remote' ? 'remoto' : 'local' }}</label>
        <input v-model.number="form.localPort" type="number" min="1" max="65535" required @blur="checkPort" />
      </div>
    </div>
    <div v-if="portStatus === 'checking'" class="port-hint checking">Comprobando el puerto…</div>
    <div v-else-if="portStatus === 'busy'" class="port-hint busy">
      ⚠ Ese puerto ya está en uso en tu equipo — el túnel probablemente no va a poder iniciar hasta que liberes el puerto o elijas otro.
    </div>
    <div v-else-if="portStatus === 'free'" class="port-hint free">✓ Puerto disponible.</div>

    <div v-if="form.kind !== 'Dynamic'" class="field-row">
      <div class="field">
        <label>Host destino</label>
        <input v-model="form.remoteHost" placeholder="db.internal" required />
      </div>
      <div class="field field-small">
        <label>Puerto destino</label>
        <input v-model.number="form.remotePort" type="number" min="1" max="65535" required />
      </div>
    </div>

    <div class="actions">
      <button type="button" class="secondary" @click="emit('cancel')">Cancelar</button>
      <button type="submit" class="primary">Guardar perfil</button>
    </div>
  </form>
</template>

<style scoped>
.tunnel-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 24px;
  background: var(--surface);
  border-radius: 12px;
  border: 1px solid var(--border);
}
.field { display: flex; flex-direction: column; gap: 7px; flex: 1; }
.field-row { display: flex; gap: 14px; }
.field-small { flex: 0 0 120px; }
label {
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
input {
  padding: 10px 12px;
  border-radius: 7px;
  border: 1px solid var(--border);
  background: var(--surface-inset);
  color: var(--text);
  font-size: 14px;
  font-family: var(--font-body);
}
input::placeholder { color: var(--text-faint); }

.input-with-button { display: flex; gap: 8px; }
.input-with-button input, .input-with-button select { flex: 1; min-width: 0; }
.browse-btn {
  padding: 10px 14px;
  border-radius: 7px;
  border: 1px solid var(--border);
  background: var(--surface-inset);
  color: var(--text-muted);
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  cursor: pointer;
}
.browse-btn:hover { color: var(--text); border-color: var(--text-faint); }
.browse-btn.danger:hover { color: var(--danger); border-color: var(--danger); }

.template-picker { padding: 12px; background: var(--surface-inset); border-radius: 8px; border: 1px solid var(--border); }

.template-actions { display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
.link-btn {
  background: none;
  border: none;
  color: var(--accent);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  padding: 2px 0;
}
.link-btn:disabled { color: var(--text-faint); cursor: default; }
.test-result {
  font-size: 13px;
  padding: 8px 12px;
  border-radius: 6px;
  background: var(--surface-inset);
  width: 100%;
  overflow-wrap: anywhere;
}
.test-result.ok { color: var(--success); }
.test-result.fail { color: var(--danger); }

.port-hint { font-size: 12px; margin-top: -8px; }
.port-hint.checking { color: var(--text-faint); }
.port-hint.busy { color: var(--warning); }
.port-hint.free { color: var(--success); }

/* Native <select> keeps its OS chrome (light "Aqua" combobox on macOS, for
   example) unless appearance is reset — otherwise the background/color you
   set here gets ignored and it renders as a pale, disabled-looking box. */
select {
  appearance: none;
  -webkit-appearance: none;
  padding: 10px 36px 10px 12px;
  border-radius: 7px;
  border: 1px solid var(--border);
  background-color: var(--surface-inset);
  color: var(--text);
  font-size: 14px;
  font-family: var(--font-body);
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'%3E%3Cpath d='M1 1l4 4 4-4' stroke='%237c8b99' stroke-width='1.5' fill='none' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
}
select option {
  background: var(--surface-inset);
  color: var(--text);
}
input:focus, select:focus { outline: 2px solid var(--accent); outline-offset: 1px; }
.actions { display: flex; justify-content: flex-end; gap: 10px; margin-top: 6px; }
button { padding: 10px 18px; border-radius: 7px; border: none; font-size: 14px; font-weight: 600; cursor: pointer; }
.primary { background: var(--accent); color: #06201c; }
.primary:hover { background: var(--accent-hover); }
.secondary { background: transparent; color: var(--text-muted); border: 1px solid var(--border); }
.secondary:hover { color: var(--text); border-color: var(--text-muted); }
</style>
