import { ref } from "vue";
import { useTunnelStore } from "../stores/tunnels";
import type { TestConnectionResult, TunnelProfile } from "../types/tunnel";

/** Drives the "Probar conexión" button: connects and authenticates against
 * the SSH host with no forwarding started, so mistakes in host/credentials
 * show up while filling out the form instead of only after saving. */
export function useConnectionTest() {
  const store = useTunnelStore();
  const testing = ref(false);
  const result = ref<TestConnectionResult | null>(null);

  async function testConnection(profile: TunnelProfile) {
    testing.value = true;
    result.value = null;
    try {
      const message = await store.testConnection({ ...profile });
      result.value = { ok: true, message };
    } catch (err) {
      result.value = { ok: false, message: String(err) };
    } finally {
      testing.value = false;
    }
  }

  return { testing, result, testConnection };
}
