import { ref } from "vue";
import { useTunnelStore } from "../stores/tunnels";
import type { TunnelProfile } from "../types/tunnel";

export function useSshTemplates(form: TunnelProfile) {
  const store = useTunnelStore();
  const selectedTemplateId = ref<string>("");
  const showSaveTemplate = ref(false);
  const newTemplateName = ref("");

  function applyTemplate() {
    const tpl = store.templates.find((t) => t.id === selectedTemplateId.value);
    if (!tpl) return;
    form.sshHost = tpl.sshHost;
    form.sshPort = tpl.sshPort;
    form.sshUser = tpl.sshUser;
    form.authMethod = tpl.authMethod;
    form.privateKeyPath = tpl.privateKeyPath || "";
  }

  async function saveAsTemplate() {
    if (!newTemplateName.value.trim()) return;
    await store.saveTemplate({
      id: "",
      name: newTemplateName.value.trim(),
      sshHost: form.sshHost,
      sshPort: form.sshPort,
      sshUser: form.sshUser,
      authMethod: form.authMethod,
      privateKeyPath: form.authMethod === "PrivateKey" ? form.privateKeyPath : undefined,
    });
    newTemplateName.value = "";
    showSaveTemplate.value = false;
  }

  async function deleteSelectedTemplate() {
    if (!selectedTemplateId.value) return;
    await store.deleteTemplate(selectedTemplateId.value);
    selectedTemplateId.value = "";
  }

  return {
    selectedTemplateId,
    showSaveTemplate,
    newTemplateName,
    applyTemplate,
    saveAsTemplate,
    deleteSelectedTemplate,
  };
}
